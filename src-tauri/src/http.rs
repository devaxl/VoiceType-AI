//! Shared HTTP client (with timeouts) and retry/backoff helpers for the OpenAI calls.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::StatusCode;

/// A process-wide reqwest client with sane connect/overall timeouts.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Total attempts (1 initial + 2 retries) for transient failures.
pub const MAX_ATTEMPTS: u32 = 3;

/// Backoff delay before the next attempt (0-indexed): 300ms, 600ms, 1200ms, …
pub fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(300u64 * (1u64 << attempt))
}

/// Whether an HTTP status is worth retrying (rate limit or server error).
pub fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}
