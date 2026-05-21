use serde::Serialize;
use std::time::Duration;

/// Structured error sent to the frontend via Tauri events.
/// The frontend uses `code` to look up an i18n-translated message.
#[derive(Debug, Clone, Serialize)]
pub struct UserError {
    pub code: String,
    pub details: Option<String>,
    pub retry_count: u32,
}

/// Internal error type used throughout the Rust backend.
/// Provides `is_retryable()` for retry logic and `to_user_error()` for frontend display.
#[derive(Debug)]
pub enum AppError {
    Network(String),
    Timeout(Duration),
    Api { status: u16, body: String },
    Auth(String),
    Output(String),
    Config(String),
}

impl AppError {
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Network(_) => true,
            AppError::Timeout(_) => true,
            AppError::Api { status, .. } => *status >= 500,
            AppError::Auth(_) => false,
            AppError::Output(_) => false,
            AppError::Config(_) => false,
        }
    }

    pub fn to_user_error(&self) -> UserError {
        let (code, details) = match self {
            AppError::Network(msg) => ("stt_timeout".to_string(), Some(msg.clone())),
            AppError::Timeout(_) => ("stt_timeout".to_string(), None),
            AppError::Api { status, body } => {
                if *status == 401 || *status == 403 {
                    ("stt_invalid_key".to_string(), None)
                } else {
                    ("stt_failed".to_string(), Some(format!("HTTP {}", status)))
                }
            }
            AppError::Auth(msg) => ("stt_invalid_key".to_string(), Some(msg.clone())),
            AppError::Output(msg) => ("output_fallback_clipboard".to_string(), Some(msg.clone())),
            AppError::Config(msg) => ("stt_failed".to_string(), Some(msg.clone())),
        };
        UserError {
            code,
            details,
            retry_count: 0,
        }
    }

    pub fn with_retry_count(self, count: u32) -> UserError {
        let mut ue = self.to_user_error();
        ue.retry_count = count;
        ue
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Network(msg) => write!(f, "Network error: {}", msg),
            AppError::Timeout(d) => write!(f, "Timeout after {:.1}s", d.as_secs_f64()),
            AppError::Api { status, body } => write!(f, "API error {}: {}", status, body),
            AppError::Auth(msg) => write!(f, "Auth error: {}", msg),
            AppError::Output(msg) => write!(f, "Output error: {}", msg),
            AppError::Config(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::Timeout(Duration::from_secs(30))
        } else if let Some(status) = e.status() {
            AppError::Api {
                status: status.as_u16(),
                body: e.to_string(),
            }
        } else {
            AppError::Network(e.to_string())
        }
    }
}

/// Retry an async operation with exponential backoff.
/// - `max_retries`: number of retries (0 = no retry)
/// - `f`: closure returning a Future that produces Result<T, AppError>
/// Emits a `pipeline:retry` event on each retry attempt.
pub async fn with_retry<F, Fut, T>(
    app_handle: &tauri::AppHandle,
    max_retries: u32,
    f: F,
) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let mut last_error: Option<AppError> = None;
    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                let delay_ms = 1000 * 2u64.pow(attempt);
                tracing::warn!(
                    "Retryable error (attempt {}/{}): {}, retrying in {}ms",
                    attempt + 1,
                    max_retries,
                    e,
                    delay_ms
                );
                let _ = app_handle.emit(
                    "pipeline:retry",
                    serde_json::json!({
                        "attempt": attempt + 1,
                        "max": max_retries,
                        "error": e.to_string(),
                    }),
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_error.unwrap())
}
