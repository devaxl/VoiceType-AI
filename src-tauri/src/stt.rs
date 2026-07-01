use crate::config::Provider;
use crate::error::{AppError, Result};
use crate::http;

/// Transcription endpoint for the given provider. Groq exposes an OpenAI-compatible
/// `/audio/transcriptions` route, so the multipart body below is identical for both.
fn endpoint(provider: Provider) -> &'static str {
    match provider {
        Provider::Groq => "https://api.groq.com/openai/v1/audio/transcriptions",
        // Anthropic has no transcription API; the UI never offers it for STT. Fall back to OpenAI.
        Provider::OpenAI | Provider::Anthropic => "https://api.openai.com/v1/audio/transcriptions",
    }
}

/// Transcribe a WAV byte buffer via the provider's transcription API, retrying transient failures.
///
/// `vocabulary` (if non-empty) is sent as the `prompt` parameter to bias recognition toward the
/// user's jargon/terms. Uses `response_format=text`, so the body is the plain transcript.
pub async fn transcribe(
    provider: Provider,
    api_key: &str,
    model: &str,
    wav: Vec<u8>,
    vocabulary: &str,
) -> Result<String> {
    let endpoint = endpoint(provider);
    let mut last_err = String::new();

    for attempt in 0..http::MAX_ATTEMPTS {
        // The multipart form is consumed per send, so rebuild it (and re-clone the audio) each try.
        let part = reqwest::multipart::Part::bytes(wav.clone())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AppError::Stt(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("response_format", "text")
            .part("file", part);
        if !vocabulary.trim().is_empty() {
            form = form.text("prompt", vocabulary.to_string());
        }

        match http::client()
            .post(endpoint)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.map_err(|e| AppError::Network(e.to_string()))?;
                return Ok(text.trim().to_string());
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if http::is_retryable_status(status) && attempt + 1 < http::MAX_ATTEMPTS {
                    last_err = format!("{status}: {body}");
                    tokio::time::sleep(http::backoff(attempt)).await;
                    continue;
                }
                return Err(AppError::Stt(format!("{status}: {body}")));
            }
            Err(e) if (e.is_timeout() || e.is_connect()) && attempt + 1 < http::MAX_ATTEMPTS => {
                last_err = e.to_string();
                tokio::time::sleep(http::backoff(attempt)).await;
                continue;
            }
            Err(e) => return Err(AppError::Network(e.to_string())),
        }
    }

    Err(AppError::Stt(format!(
        "transcription failed after retries: {last_err}"
    )))
}
