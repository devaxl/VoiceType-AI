use serde_json::json;

use crate::config::Provider;
use crate::error::{AppError, Result};
use crate::http;

const OPENAI_CHAT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const GROQ_CHAT_ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const ANTHROPIC_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Cap on refined output. A 60-second dictation is only a few hundred tokens, so this is generous
/// headroom that still bounds a runaway response (Anthropic requires an explicit `max_tokens`).
const MAX_TOKENS: u32 = 2048;

/// Refine a raw transcript into clean text via the selected provider's API, retrying transient
/// failures.
///
/// The transcript is wrapped in delimiters and the system prompt instructs the model to treat it
/// as data, not instructions — a basic guard against spoken-content prompt injection. The caller
/// falls back to the raw transcript on any error, so refinement never loses the user's words.
pub async fn refine(
    provider: Provider,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    transcript: &str,
) -> Result<String> {
    match provider {
        Provider::Anthropic => refine_anthropic(api_key, model, system_prompt, transcript).await,
        Provider::OpenAI => {
            refine_chat(OPENAI_CHAT_ENDPOINT, api_key, model, system_prompt, transcript).await
        }
        Provider::Groq => {
            refine_chat(GROQ_CHAT_ENDPOINT, api_key, model, system_prompt, transcript).await
        }
    }
}

/// OpenAI-compatible chat-completions path (OpenAI and Groq share this request/response shape).
async fn refine_chat(
    endpoint: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    transcript: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": format!("<transcript>\n{transcript}\n</transcript>") }
        ],
        "temperature": 0.3
    });

    let mut last_err = String::new();

    for attempt in 0..http::MAX_ATTEMPTS {
        match http::client()
            .post(endpoint)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let value: serde_json::Value =
                    resp.json().await.map_err(|e| AppError::Refine(e.to_string()))?;
                let content = value["choices"][0]["message"]["content"]
                    .as_str()
                    .ok_or_else(|| AppError::Refine("unexpected response shape".into()))?;
                return Ok(content.trim().to_string());
            }
            Ok(resp) => {
                let status = resp.status();
                let detail = resp.text().await.unwrap_or_default();
                if http::is_retryable_status(status) && attempt + 1 < http::MAX_ATTEMPTS {
                    last_err = format!("{status}: {detail}");
                    tokio::time::sleep(http::backoff(attempt)).await;
                    continue;
                }
                return Err(AppError::Refine(format!("{status}: {detail}")));
            }
            Err(e) if (e.is_timeout() || e.is_connect()) && attempt + 1 < http::MAX_ATTEMPTS => {
                last_err = e.to_string();
                tokio::time::sleep(http::backoff(attempt)).await;
                continue;
            }
            Err(e) => return Err(AppError::Network(e.to_string())),
        }
    }

    Err(AppError::Refine(format!(
        "refinement failed after retries: {last_err}"
    )))
}

/// Anthropic Messages API path. Differs from chat-completions: `x-api-key` + `anthropic-version`
/// headers, `max_tokens` is required, the system prompt is a top-level field, and the reply is a
/// list of content blocks. `temperature` is omitted so the request is valid on every Claude model
/// (newer models reject non-default sampling params).
async fn refine_anthropic(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    transcript: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": system_prompt,
        "messages": [
            { "role": "user", "content": format!("<transcript>\n{transcript}\n</transcript>") }
        ]
    });

    let mut last_err = String::new();

    for attempt in 0..http::MAX_ATTEMPTS {
        match http::client()
            .post(ANTHROPIC_ENDPOINT)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let value: serde_json::Value =
                    resp.json().await.map_err(|e| AppError::Refine(e.to_string()))?;
                let text = first_text_block(&value)
                    .ok_or_else(|| AppError::Refine("unexpected response shape".into()))?;
                return Ok(text.trim().to_string());
            }
            Ok(resp) => {
                let status = resp.status();
                let detail = resp.text().await.unwrap_or_default();
                if http::is_retryable_status(status) && attempt + 1 < http::MAX_ATTEMPTS {
                    last_err = format!("{status}: {detail}");
                    tokio::time::sleep(http::backoff(attempt)).await;
                    continue;
                }
                return Err(AppError::Refine(format!("{status}: {detail}")));
            }
            Err(e) if (e.is_timeout() || e.is_connect()) && attempt + 1 < http::MAX_ATTEMPTS => {
                last_err = e.to_string();
                tokio::time::sleep(http::backoff(attempt)).await;
                continue;
            }
            Err(e) => return Err(AppError::Network(e.to_string())),
        }
    }

    Err(AppError::Refine(format!(
        "refinement failed after retries: {last_err}"
    )))
}

/// Extract the first text block from an Anthropic `messages` response
/// (`content: [{ "type": "text", "text": "…" }, …]`).
fn first_text_block(value: &serde_json::Value) -> Option<&str> {
    value["content"]
        .as_array()?
        .iter()
        .find(|block| block["type"] == "text")
        .and_then(|block| block["text"].as_str())
}
