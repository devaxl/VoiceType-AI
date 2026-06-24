use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::config::MAX_RECORDING_SECS;
use crate::error::{AppError, Result};

/// A finished recording: interleaved 16-bit PCM at the device's native rate.
pub struct Recording {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Owns the lifetime of an active capture. The cpal `Stream` is not `Send`, so it lives entirely
/// inside a dedicated thread; we communicate with that thread over channels.
#[derive(Default)]
pub struct Recorder {
    active: Option<Active>,
}

struct Active {
    stop_tx: Sender<()>,
    handle: JoinHandle<Result<Recording>>,
}

impl Recorder {
    #[allow(dead_code)] // used by a planned status-sync path
    pub fn is_recording(&self) -> bool {
        self.active.is_some()
    }

    pub fn start(&mut self) -> Result<()> {
        if self.active.is_some() {
            return Ok(());
        }
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || capture_loop(stop_rx));
        self.active = Some(Active { stop_tx, handle });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Recording> {
        let active = self
            .active
            .take()
            .ok_or_else(|| AppError::Audio("not recording".into()))?;
        // Signal the capture thread to finish; ignore send errors (thread may have already exited
        // on the max-duration timeout).
        let _ = active.stop_tx.send(());
        active
            .handle
            .join()
            .map_err(|_| AppError::Audio("recorder thread panicked".into()))?
    }
}

fn capture_loop(stop_rx: Receiver<()>) -> Result<Recording> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AppError::NoInputDevice)?;
    let supported = device
        .default_input_config()
        .map_err(|e| AppError::Audio(e.to_string()))?;

    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();
    let sample_format = supported.sample_format();
    let stream_config: cpal::StreamConfig = supported.into();

    let buffer = Arc::new(Mutex::new(Vec::<i16>::new()));
    let err_fn = |e| eprintln!("audio stream error: {e}");

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let buf = buffer.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16));
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let buf = buffer.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    buf.lock().unwrap().extend_from_slice(data);
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let buf = buffer.clone();
            device.build_input_stream(
                &stream_config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&s| (s as i32 - 32768) as i16));
                },
                err_fn,
                None,
            )
        }
        other => return Err(AppError::Audio(format!("unsupported sample format: {other:?}"))),
    }
    .map_err(|e| AppError::Audio(e.to_string()))?;

    stream.play().map_err(|e| AppError::Audio(e.to_string()))?;

    // Run until the stop signal arrives, or hard-stop at the max recording duration.
    let _ = stop_rx.recv_timeout(Duration::from_secs(MAX_RECORDING_SECS));
    drop(stream);

    let samples = match Arc::try_unwrap(buffer) {
        Ok(mutex) => mutex.into_inner().unwrap(),
        Err(arc) => arc.lock().unwrap().clone(),
    };

    Ok(Recording {
        samples,
        sample_rate,
        channels,
    })
}

/// Encode a recording as a 16-bit PCM WAV in memory (what the OpenAI transcription API accepts).
pub fn to_wav(recording: &Recording) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: recording.channels,
        sample_rate: recording.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| AppError::Audio(e.to_string()))?;
        for &sample in &recording.samples {
            writer
                .write_sample(sample)
                .map_err(|e| AppError::Audio(e.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|e| AppError::Audio(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

/// Heuristic minimum-duration gate (~300ms of interleaved samples) to drop accidental taps.
pub fn is_too_short(recording: &Recording) -> bool {
    let min_samples = (recording.sample_rate as usize) * (recording.channels as usize) * 3 / 10;
    recording.samples.len() < min_samples
}
