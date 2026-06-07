//! Error recovery module with enhanced exponential backoff and jitter
//!
//! This module provides error recovery strategies with jitter support.
//! For simpler retry logic without jitter, use `crate::recovery::retry` instead.

use crate::error::{Paper2CodesError, Result};
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Error recovery strategies with enhanced exponential backoff and jitter
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Retry an operation with exponential backoff and jitter
    pub async fn retry_with_backoff<F, T>(
        mut operation: F,
        max_retries: u32,
        initial_delay_ms: u64,
    ) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut delay = initial_delay_ms;
        let mut last_error = None;
        let max_delay_ms = 30000; // 30 seconds max delay

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    // Store error message since we can't clone the error
                    last_error = Some(format!("{}", e));

                    if !e.is_retryable() {
                        warn!("Non-retryable error encountered: {}", e);
                        return Err(e);
                    }

                    if attempt < max_retries {
                        // Exponential backoff with jitter to prevent thundering herd
                        let jitter = rand::thread_rng().gen_range(0..=delay / 4);
                        let backoff_delay = (delay + jitter).min(max_delay_ms);

                        warn!(
                            "Retry attempt {}/{} after {}ms delay: {}",
                            attempt + 1,
                            max_retries + 1,
                            backoff_delay,
                            e
                        );

                        sleep(Duration::from_millis(backoff_delay)).await;
                        delay = (delay * 2).min(max_delay_ms); // Exponential backoff with cap
                    }
                }
            }
        }

        Err(Paper2CodesError::Validation(
            last_error.unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }

    /// Retry with exponential backoff and custom max delay
    pub async fn retry_with_backoff_custom<F, T>(
        mut operation: F,
        max_retries: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut delay = initial_delay_ms;
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        debug!("Operation succeeded after {} retries", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(format!("{}", e));

                    if !e.is_retryable() {
                        return Err(e);
                    }

                    if attempt < max_retries {
                        let jitter = rand::thread_rng().gen_range(0..=delay / 4);
                        let backoff_delay = (delay + jitter).min(max_delay_ms);

                        debug!(
                            "Retry attempt {}/{} after {}ms",
                            attempt + 1,
                            max_retries + 1,
                            backoff_delay
                        );
                        sleep(Duration::from_millis(backoff_delay)).await;
                        delay = (delay * 2).min(max_delay_ms);
                    }
                }
            }
        }

        Err(Paper2CodesError::Validation(
            last_error.unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }

    /// Retry with fixed delay
    pub async fn retry_with_fixed_delay<F, T>(
        mut operation: F,
        max_retries: u32,
        delay_ms: u64,
    ) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Store error message since we can't clone the error
                    last_error = Some(format!("{}", e));

                    if !e.is_retryable() {
                        return Err(e);
                    }

                    if attempt < max_retries {
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(Paper2CodesError::Validation(
            last_error.unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }

    /// Handle rate limit errors with appropriate delay (longer backoff)
    pub async fn handle_rate_limit<F, T>(operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        const MAX_RETRIES: u32 = 5;
        const INITIAL_DELAY_MS: u64 = 2000; // Start with 2 seconds for rate limits
        const MAX_DELAY_MS: u64 = 60000; // Max 60 seconds

        Self::retry_with_backoff_custom(operation, MAX_RETRIES, INITIAL_DELAY_MS, MAX_DELAY_MS)
            .await
    }

    /// Handle network errors with retry
    pub async fn handle_network_error<F, T>(operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_DELAY_MS: u64 = 500;
        const MAX_DELAY_MS: u64 = 5000; // Max 5 seconds for network errors

        Self::retry_with_backoff_custom(operation, MAX_RETRIES, INITIAL_DELAY_MS, MAX_DELAY_MS)
            .await
    }

    /// Handle timeout errors with retry
    pub async fn handle_timeout<F, T>(operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        const MAX_RETRIES: u32 = 2;
        const INITIAL_DELAY_MS: u64 = 1000;
        const MAX_DELAY_MS: u64 = 10000;

        Self::retry_with_backoff_custom(operation, MAX_RETRIES, INITIAL_DELAY_MS, MAX_DELAY_MS)
            .await
    }
}

/// Helper macro for retrying operations
#[macro_export]
macro_rules! retry {
    ($operation:expr, $max_retries:expr, $delay_ms:expr) => {
        $crate::error::recovery::ErrorRecovery::retry_with_fixed_delay(
            || Box::pin(async { $operation }),
            $max_retries,
            $delay_ms,
        )
        .await
    };
}
