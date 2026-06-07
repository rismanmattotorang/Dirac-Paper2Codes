//! Retry mechanisms with exponential backoff

use crate::error::{Paper2CodesError, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Configuration for retry operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            multiplier: 2.0,
        }
    }
}

/// Retry an operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T>(mut operation: F, config: RetryConfig) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = config.base_delay_ms;
    let mut last_error: Option<String> = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(format!("{}", e));

                // Check if error is retryable
                if !is_retryable_error(&e) {
                    return Err(e);
                }

                if attempt < config.max_retries {
                    let actual_delay = delay.min(config.max_delay_ms);
                    warn!(
                        "Operation failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        config.max_retries + 1,
                        actual_delay,
                        last_error.as_ref().unwrap()
                    );
                    sleep(Duration::from_millis(actual_delay)).await;
                    delay = ((delay as f64) * config.multiplier) as u64;
                }
            }
        }
    }

    Err(Paper2CodesError::Validation(format!(
        "Max retries ({}) exceeded. Last error: {}",
        config.max_retries,
        last_error.unwrap_or_else(|| "Unknown error".to_string())
    )))
}

/// Check if an error is retryable
fn is_retryable_error(error: &Paper2CodesError) -> bool {
    use crate::error::Paper2CodesError::*;
    match error {
        LLM(err) => matches!(
            err,
            crate::error::LLMError::RateLimit | crate::error::LLMError::Network(_)
        ),
        _ => false, // Other errors may or may not be retryable
    }
}

/// Retry with default configuration
pub async fn retry_default<F, Fut, T>(operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    retry_with_backoff(operation, RetryConfig::default()).await
}
