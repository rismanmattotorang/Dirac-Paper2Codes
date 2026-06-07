//! Error recovery mechanisms with exponential backoff and fallback strategies

pub mod fallback;
pub mod retry;

pub use fallback::{execute_with_fallback, FallbackStrategy};
pub use retry::{retry_with_backoff, RetryConfig};
