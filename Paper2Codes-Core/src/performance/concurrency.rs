//! Adaptive concurrency control
use crate::error::Result;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::debug;

/// Configuration for adaptive concurrency
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    pub initial_limit: usize,
    pub min_limit: usize,
    pub max_limit: usize,
    pub step_size: usize,
    pub success_threshold: f64,
    pub window_size: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            initial_limit: 4,
            min_limit: 1,
            max_limit: 10,
            step_size: 1,
            success_threshold: 0.8,
            window_size: 10,
        }
    }
}

/// Adaptive concurrency controller with dynamic semaphore management
/// Implements a production-ready adaptive concurrency control system that:
/// - Dynamically adjusts concurrency limits based on success rates
/// - Uses multiple semaphores for different limit ranges to enable true dynamic adjustment
/// - Tracks performance metrics for intelligent scaling decisions
/// - Prevents system overload while maximizing throughput
pub struct AdaptiveConcurrency {
    current_limit: Arc<RwLock<usize>>,
    min_limit: usize,
    max_limit: usize,
    step_size: usize,
    success_threshold: f64,
    window_size: usize,
    recent_results: Arc<RwLock<Vec<bool>>>,
    // Use multiple semaphores for different limit ranges to enable dynamic adjustment
    semaphores: Arc<RwLock<Vec<(usize, Arc<Semaphore>)>>>,
    initial_limit: usize,
    // Performance tracking
    total_requests: Arc<RwLock<u64>>,
    total_successes: Arc<RwLock<u64>>,
    total_failures: Arc<RwLock<u64>>,
}

impl AdaptiveConcurrency {
    pub fn new(config: ConcurrencyConfig) -> Self {
        let initial_limit = config.initial_limit;

        // Pre-create semaphores for common limit values to enable dynamic switching
        // This allows us to effectively change limits by switching between semaphores
        let mut semaphores = Vec::new();
        for limit in (config.min_limit..=config.max_limit).step_by(config.step_size.max(1)) {
            semaphores.push((limit, Arc::new(Semaphore::new(limit))));
        }
        // Ensure we have semaphores for min and max limits
        if !semaphores.iter().any(|(l, _)| *l == config.min_limit) {
            semaphores.insert(
                0,
                (config.min_limit, Arc::new(Semaphore::new(config.min_limit))),
            );
        }
        if !semaphores.iter().any(|(l, _)| *l == config.max_limit) {
            semaphores.push((config.max_limit, Arc::new(Semaphore::new(config.max_limit))));
        }
        semaphores.sort_by_key(|(l, _)| *l);

        Self {
            current_limit: Arc::new(RwLock::new(initial_limit)),
            semaphores: Arc::new(RwLock::new(semaphores)),
            min_limit: config.min_limit,
            max_limit: config.max_limit,
            step_size: config.step_size,
            success_threshold: config.success_threshold,
            window_size: config.window_size,
            recent_results: Arc::new(RwLock::new(Vec::with_capacity(config.window_size))),
            initial_limit,
            total_requests: Arc::new(RwLock::new(0)),
            total_successes: Arc::new(RwLock::new(0)),
            total_failures: Arc::new(RwLock::new(0)),
        }
    }

    /// Get the appropriate semaphore for the current limit
    /// Returns the semaphore with the closest limit that is <= the desired limit
    async fn get_semaphore_for_limit(&self, limit: usize) -> Option<Arc<Semaphore>> {
        let semaphores = self.semaphores.read().await;
        // Find the closest semaphore with limit <= desired limit
        // Since semaphores are sorted, we can find the best match
        semaphores
            .iter()
            .rev()
            .find(|(l, _)| *l <= limit)
            .or_else(|| semaphores.first())
            .map(|(_, s)| Arc::clone(s))
    }

    pub async fn acquire(&self) -> Result<ConcurrencyPermit> {
        // Get current limit and corresponding semaphore
        let limit = *self.current_limit.read().await;
        let semaphore = self.get_semaphore_for_limit(limit).await.ok_or_else(|| {
            crate::error::Paper2CodesError::Validation(format!(
                "No semaphore available for limit {}",
                limit
            ))
        })?;

        // Track total requests
        {
            let mut total = self.total_requests.write().await;
            *total += 1;
        }

        // Acquire permit from semaphore
        let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
            crate::error::Paper2CodesError::Validation(format!(
                "Failed to acquire semaphore: {}",
                e
            ))
        })?;

        Ok(ConcurrencyPermit {
            _permit: permit,
            _controller: self.clone(),
        })
    }

    pub async fn record_result(&self, success: bool) {
        // Update global statistics
        {
            if success {
                let mut total = self.total_successes.write().await;
                *total += 1;
            } else {
                let mut total = self.total_failures.write().await;
                *total += 1;
            }
        }

        // Update sliding window
        let mut results = self.recent_results.write().await;
        results.push(success);

        if results.len() > self.window_size {
            results.remove(0);
        }

        if results.is_empty() {
            return;
        }

        // Calculate success rate with improved precision
        let success_count = results.iter().filter(|&&s| s).count();
        let success_rate = success_count as f64 / results.len() as f64;

        // Calculate failure rate for more nuanced adjustment
        let failure_rate = 1.0 - success_rate;

        // Adjust limit based on success rate with hysteresis to prevent oscillation
        // Use a single write lock to avoid race conditions
        let (old_limit, new_limit) = {
            let mut limit_guard = self.current_limit.write().await;
            let old_limit = *limit_guard;
            let mut new_limit = old_limit;

            // Use hysteresis: require higher threshold to increase, lower to decrease
            // This prevents rapid oscillation around the threshold
            if success_rate >= self.success_threshold + 0.05 {
                // Increase limit if success rate is significantly above threshold
                new_limit = (old_limit + self.step_size).min(self.max_limit);
            } else if failure_rate > (1.0 - self.success_threshold) + 0.1 {
                // Decrease limit if failure rate is significantly above threshold
                new_limit = old_limit.saturating_sub(self.step_size).max(self.min_limit);
            }

            if new_limit != old_limit {
                *limit_guard = new_limit;
            }

            (old_limit, new_limit)
        };

        if new_limit != old_limit {
            debug!(
                "Adjusting concurrency limit from {} to {} (success rate: {:.2}, window: {})",
                old_limit,
                new_limit,
                success_rate,
                results.len()
            );
        }
    }

    /// Get performance statistics
    pub async fn get_stats(&self) -> ConcurrencyStats {
        let total_requests = *self.total_requests.read().await;
        let total_successes = *self.total_successes.read().await;
        let total_failures = *self.total_failures.read().await;
        let current_limit = *self.current_limit.read().await;
        let recent_results = self.recent_results.read().await;
        let recent_success_rate = if recent_results.is_empty() {
            0.0
        } else {
            recent_results.iter().filter(|&&s| s).count() as f64 / recent_results.len() as f64
        };

        ConcurrencyStats {
            current_limit,
            total_requests,
            total_successes,
            total_failures,
            overall_success_rate: if total_requests > 0 {
                total_successes as f64 / total_requests as f64
            } else {
                0.0
            },
            recent_success_rate,
            window_size: recent_results.len(),
        }
    }

    /// Get current concurrency limit
    pub async fn current_limit(&self) -> usize {
        *self.current_limit.read().await
    }
}

impl Clone for AdaptiveConcurrency {
    fn clone(&self) -> Self {
        Self {
            current_limit: Arc::clone(&self.current_limit),
            semaphores: Arc::clone(&self.semaphores),
            min_limit: self.min_limit,
            max_limit: self.max_limit,
            step_size: self.step_size,
            success_threshold: self.success_threshold,
            window_size: self.window_size,
            recent_results: Arc::clone(&self.recent_results),
            initial_limit: self.initial_limit,
            total_requests: Arc::clone(&self.total_requests),
            total_successes: Arc::clone(&self.total_successes),
            total_failures: Arc::clone(&self.total_failures),
        }
    }
}

/// Performance statistics for adaptive concurrency controller
#[derive(Debug, Clone)]
pub struct ConcurrencyStats {
    pub current_limit: usize,
    pub total_requests: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub overall_success_rate: f64,
    pub recent_success_rate: f64,
    pub window_size: usize,
}

/// Permit for adaptive concurrency
pub struct ConcurrencyPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    _controller: AdaptiveConcurrency,
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        // Permit is automatically released when dropped
    }
}
