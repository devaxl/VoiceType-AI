use serde_json::json;

use crate::error::{AppError, Result};
use crate::http;

const ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// Refine a raw transcript via the OpenAI chat completions API, retrying transient failures.
///
/// The transcript is wrapped in delimiters and the system prompt instructs the model to treat it
/// as data, not instructions — a basic guard against spoken-content prompt injection.
pub async fn refine(
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
            .post(ENDPOINT)
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
