# Data handling

VoiceType AI sends your dictated audio and its transcript to third-party APIs to function. This
document is the user-facing disclosure surfaced from Settings.

## What is sent, and where

| Stage | Data leaving the device | Destination |
|-------|-------------------------|-------------|
| Transcription | The recorded audio (WAV, in memory) | OpenAI `/v1/audio/transcriptions` |
| Refinement | The transcribed text | OpenAI `/v1/chat/completions` |

- **Audio is held in memory only** and is never written to disk by the app.
- The OpenAI API key is stored in the **OS keychain** (Windows Credential Manager / macOS
  Keychain), never in a plaintext config file.

## Requirements for sensitive content

Dictation can contain secrets, customer data, or PII. Before using this for sensitive content:

- Use an **organization-managed OpenAI workspace** with a signed **Zero-Data-Retention (ZDR) +
  no-training** agreement (and a DPA), covering both the transcription and chat endpoints.
- Confirm **data residency** (EU/UK users may require regional endpoints or a documented transfer
  basis).

## Per-user-keys trade-off

This build uses **per-user API keys** (no central proxy). That means there is **no central**
rotation, revocation, cost cap, or retention enforcement — those are managed in each user's provider
workspace. On offboarding, revoke the user's key in the OpenAI console; the app clears its keychain
entry on uninstall.

See `PRD.md` §9 (Security, Privacy & Permissions) for the full posture.
