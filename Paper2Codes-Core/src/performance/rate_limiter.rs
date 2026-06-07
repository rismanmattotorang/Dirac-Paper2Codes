//! Smart rate limiting with token awareness
use crate::error::{Paper2CodesError, Result};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_rpm: u32,
    pub tokens_per_minute: u32,
    pub max_concurrent: usize,
    pub adaptive: bool,
    pub initial_delay_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_rpm: 60,
            tokens_per_minute: 60000,
            max_concurrent: 5,
            adaptive: true,
            initial_delay_ms: 100,
        }
    }
}

/// Smart rate limiter with token awareness
pub struct SmartRateLimiter {
    max_rpm: u32,
    tokens_per_minute: u32,
    request_history: Arc<RwLock<VecDeque<Instant>>>,
    token_history: Arc<RwLock<VecDeque<(Instant, u32)>>>,
    semaphore: Arc<tokio::sync::Semaphore>,
    adaptive_factor: Arc<RwLock<f64>>,
    adaptive: bool,
    last_rate_limit: Arc<RwLock<Option<Instant>>>,
}

impl SmartRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            max_rpm: config.max_rpm,
            tokens_per_minute: config.tokens_per_minute,
            request_history: Arc::new(RwLock::new(VecDeque::with_capacity(
                config.max_rpm as usize * 2,
            ))),
            token_history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            semaphore: Arc::new(tokio::sync::Semaphore::new(config.max_concurrent)),
            adaptive_factor: Arc::new(RwLock::new(1.0)),
            adaptive: config.adaptive,
            last_rate_limit: Arc::new(RwLock::new(None)),
        }
    }

    /// Acquire a permit for a request, checking both RPM and token limits
    pub async fn acquire(&self, estimated_tokens: Option<u32>) -> Result<RateLimitPermit> {
        const MAX_ITERATIONS: u32 = 1000; // Prevent infinite loops
        let mut iterations = 0;

        loop {
            if iterations >= MAX_ITERATIONS {
                return Err(Paper2CodesError::Validation(
                    "Rate limit acquire exceeded maximum iterations".to_string(),
                ));
            }
            iterations += 1;

            let permit = self.semaphore.clone().acquire_owned().await.map_err(|_| {
                Paper2CodesError::Validation("Failed to acquire rate limit permit".to_string())
            })?;

            let now = Instant::now();

            // Check request rate limit
            let wait_time = self.check_request_rate_limit(now).await?;
            if wait_time > Duration::ZERO {
                drop(permit);
                debug!(
                    "Rate limit: waiting {}ms for request rate limit",
                    wait_time.as_millis()
                );
                tokio::time::sleep(wait_time).await;
                continue;
            }

            // Check token rate limit if tokens are provided
            if let Some(tokens) = estimated_tokens {
                let token_wait = self.check_token_rate_limit(now, tokens).await?;
                if token_wait > Duration::ZERO {
                    drop(permit);
                    debug!(
                        "Rate limit: waiting {}ms for token rate limit ({} tokens)",
                        token_wait.as_millis(),
                        tokens
                    );
                    tokio::time::sleep(token_wait).await;
                    continue;
                }
            }

            // Record request
            {
                let mut history = self.request_history.write().await;
                history.push_back(now);
            }

            return Ok(RateLimitPermit {
                _permit: permit,
                limiter: self.clone(),
                tokens: estimated_tokens.unwrap_or(0),
                request_time: now,
            });
        }
    }

    /// Check request rate limit and return wait time if needed
    async fn check_request_rate_limit(&self, now: Instant) -> Result<Duration> {
        let mut history = self.request_history.write().await;

        // Remove old entries (> 1 minute)
        while let Some(&oldest) = history.front() {
            if now.duration_since(oldest) > Duration::from_secs(60) {
                history.pop_front();
            } else {
                break;
            }
        }

        // Check if we're at the limit
        let effective_limit = if self.adaptive {
            let factor = *self.adaptive_factor.read().await;
            (self.max_rpm as f64 * factor) as u32
        } else {
            self.max_rpm
        };

        if history.len() >= effective_limit as usize {
            if let Some(&oldest) = history.front() {
                let elapsed = now.duration_since(oldest);
                if elapsed < Duration::from_secs(60) {
                    let wait_time = Duration::from_secs(60) - elapsed;
                    return Ok(wait_time);
                }
            }
        }

        Ok(Duration::ZERO)
    }

    /// Check token rate limit and return wait time if needed
    async fn check_token_rate_limit(&self, now: Instant, tokens: u32) -> Result<Duration> {
        let mut token_history = self.token_history.write().await;

        // Remove old entries (> 1 minute)
        while let Some(&(timestamp, _)) = token_history.front() {
            if now.duration_since(timestamp) > Duration::from_secs(60) {
                token_history.pop_front();
            } else {
                break;
            }
        }

        // Calculate total tokens used in the last minute
        let total_tokens: u32 = token_history.iter().map(|(_, t)| t).sum();

        let effective_token_limit = if self.adaptive {
            let factor = *self.adaptive_factor.read().await;
            (self.tokens_per_minute as f64 * factor) as u32
        } else {
            self.tokens_per_minute
        };

        if total_tokens + tokens > effective_token_limit {
            // Find the oldest entry we need to wait for
            if let Some(&(oldest_timestamp, _)) = token_history.front() {
                let elapsed = now.duration_since(oldest_timestamp);
                if elapsed < Duration::from_secs(60) {
                    let wait_time = Duration::from_secs(60) - elapsed;
                    return Ok(wait_time);
                }
            }
        }

        Ok(Duration::ZERO)
    }

    /// Record tokens used after a request completes
    pub async fn record_tokens(&self, tokens: u32, request_time: Instant) {
        let mut token_history = self.token_history.write().await;
        token_history.push_back((request_time, tokens));

        // Cleanup old entries
        let now = Instant::now();
        while let Some(&(timestamp, _)) = token_history.front() {
            if now.duration_since(timestamp) > Duration::from_secs(60) {
                token_history.pop_front();
            } else {
                break;
            }
        }
    }

    /// Adjust adaptive factor based on rate limit errors
    pub async fn adjust_for_rate_limit(&self) {
        if !self.adaptive {
            return;
        }

        let mut factor = *self.adaptive_factor.read().await;
        let mut last_limit = self.last_rate_limit.write().await;

        // Reduce factor if we hit rate limits frequently
        if let Some(last) = *last_limit {
            if Instant::now().duration_since(last) < Duration::from_secs(10) {
                factor = (factor * 0.9).max(0.5);
                warn!(
                    "Reducing rate limit factor to {:.2} due to frequent rate limits",
                    factor
                );
            }
        }

        *last_limit = Some(Instant::now());
        *self.adaptive_factor.write().await = factor;
    }

    /// Get current statistics
    pub async fn stats(&self) -> RateLimitStats {
        let request_history = self.request_history.read().await;
        let token_history = self.token_history.read().await;
        let factor = *self.adaptive_factor.read().await;

        let now = Instant::now();
        let requests_last_minute = request_history
            .iter()
            .filter(|&&t| now.duration_since(t) <= Duration::from_secs(60))
            .count();

        let tokens_last_minute: u32 = token_history
            .iter()
            .filter(|&&(t, _)| now.duration_since(t) <= Duration::from_secs(60))
            .map(|(_, tokens)| tokens)
            .sum();

        RateLimitStats {
            requests_last_minute: requests_last_minute as u32,
            tokens_last_minute,
            max_rpm: self.max_rpm,
            max_tokens_per_minute: self.tokens_per_minute,
            adaptive_factor: factor,
        }
    }
}

impl Clone for SmartRateLimiter {
    fn clone(&self) -> Self {
        Self {
            max_rpm: self.max_rpm,
            tokens_per_minute: self.tokens_per_minute,
            request_history: Arc::clone(&self.request_history),
            token_history: Arc::clone(&self.token_history),
            semaphore: Arc::clone(&self.semaphore),
            adaptive_factor: Arc::clone(&self.adaptive_factor),
            adaptive: self.adaptive,
            last_rate_limit: Arc::clone(&self.last_rate_limit),
        }
    }
}

/// Permit for rate-limited operations
pub struct RateLimitPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    limiter: SmartRateLimiter,
    tokens: u32,
    request_time: Instant,
}

impl RateLimitPermit {
    /// Update the token count for this request
    pub fn set_tokens(&mut self, tokens: u32) {
        self.tokens = tokens;
    }

    /// Get the estimated tokens
    pub fn tokens(&self) -> u32 {
        self.tokens
    }
}

impl Drop for RateLimitPermit {
    fn drop(&mut self) {
        // Record tokens when permit is dropped
        // Use blocking spawn to ensure tokens are recorded even if runtime is shutting down
        if self.tokens > 0 {
            let limiter = self.limiter.clone();
            let tokens = self.tokens;
            let request_time = self.request_time;

            // Try to spawn, but don't block if runtime is unavailable
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let _ = handle.spawn(async move {
                    limiter.record_tokens(tokens, request_time).await;
                });
            } else {
                // If no runtime, we can't record tokens, but this is acceptable
                // as the rate limiter will still function correctly
                tracing::debug!("Cannot record tokens: no tokio runtime available");
            }
        }
    }
}

/// Statistics for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub requests_last_minute: u32,
    pub tokens_last_minute: u32,
    pub max_rpm: u32,
    pub max_tokens_per_minute: u32,
    pub adaptive_factor: f64,
}
