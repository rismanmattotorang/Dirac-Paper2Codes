//! Fallback strategies for error recovery

use crate::error::Result;

/// Fallback strategy for handling failures
pub enum FallbackStrategy<T> {
    /// Use a fallback operation if primary fails
    UseFallback {
        fallback: Box<
            dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>
                + Send
                + Sync,
        >,
    },
    /// Return default value on failure
    UseDefault { default: T },
    /// Skip operation and continue
    Skip,
    /// Fail immediately
    Fail,
}

/// Execute an operation with fallback strategy
pub async fn execute_with_fallback<F, Fut, T>(
    operation: F,
    strategy: FallbackStrategy<T>,
) -> Result<T>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T>> + Send,
    T: Send + Sync + Clone,
{
    match operation().await {
        Ok(result) => Ok(result),
        Err(e) => match strategy {
            FallbackStrategy::UseFallback { fallback } => {
                tracing::warn!("Primary operation failed, using fallback: {}", e);
                fallback().await
            }
            FallbackStrategy::UseDefault { default } => {
                tracing::warn!("Primary operation failed, using default: {}", e);
                Ok(default)
            }
            FallbackStrategy::Skip => {
                tracing::info!("Operation failed, skipping: {}", e);
                Err(e)
            }
            FallbackStrategy::Fail => Err(e),
        },
    }
}
