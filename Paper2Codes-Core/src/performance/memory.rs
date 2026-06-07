//! Memory profiling and optimization utilities
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub heap_used_mb: f64,
    pub heap_peak_mb: f64,
    pub allocations: u64,
    pub deallocations: u64,
    pub timestamp: Instant,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            heap_used_mb: 0.0,
            heap_peak_mb: 0.0,
            allocations: 0,
            deallocations: 0,
            timestamp: Instant::now(),
        }
    }
}

/// Memory profiler for tracking and optimizing memory usage
pub struct MemoryProfiler {
    stats: Arc<RwLock<MemoryStats>>,
    peak_memory: Arc<RwLock<f64>>,
    allocation_history: Arc<RwLock<Vec<(Instant, f64)>>>,
    enabled: bool,
    last_record_time: Arc<RwLock<Instant>>,
    sampling_interval: Duration,
}

impl MemoryProfiler {
    pub fn new(enabled: bool) -> Self {
        Self {
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            peak_memory: Arc::new(RwLock::new(0.0)),
            allocation_history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            enabled,
            last_record_time: Arc::new(RwLock::new(Instant::now())),
            sampling_interval: Duration::from_secs(5), // Sample every 5 seconds
        }
    }

    /// Create with custom sampling interval
    pub fn with_sampling_interval(enabled: bool, interval_secs: u64) -> Self {
        Self {
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            peak_memory: Arc::new(RwLock::new(0.0)),
            allocation_history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            enabled,
            last_record_time: Arc::new(RwLock::new(Instant::now())),
            sampling_interval: Duration::from_secs(interval_secs),
        }
    }

    /// Record memory usage with sampling to reduce overhead
    pub async fn record_usage(&self, heap_used_mb: f64) {
        if !self.enabled {
            return;
        }

        let now = Instant::now();
        let should_sample = {
            let last_time = self.last_record_time.read().await;
            now.duration_since(*last_time) >= self.sampling_interval
        };

        // Always update current stats
        let mut stats = self.stats.write().await;
        stats.heap_used_mb = heap_used_mb;
        stats.timestamp = now;

        if heap_used_mb > stats.heap_peak_mb {
            stats.heap_peak_mb = heap_used_mb;
        }

        let mut peak = self.peak_memory.write().await;
        if heap_used_mb > *peak {
            *peak = heap_used_mb;
        }
        drop(peak);
        drop(stats);

        // Only record in history if sampling interval has passed
        if should_sample {
            let mut last_time = self.last_record_time.write().await;
            // Double-check after acquiring write lock
            if now.duration_since(*last_time) >= self.sampling_interval {
                *last_time = now;

                // Record in history
                let mut history = self.allocation_history.write().await;
                history.push((now, heap_used_mb));

                // Keep only last 1000 entries (use VecDeque for better performance)
                if history.len() > 1000 {
                    history.remove(0);
                }
            }
        }
    }

    /// Get current memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        self.stats.read().await.clone()
    }

    /// Get peak memory usage
    pub async fn get_peak_memory(&self) -> f64 {
        *self.peak_memory.read().await
    }

    /// Check if memory usage is high and suggest optimization
    pub async fn check_memory_pressure(&self, threshold_mb: f64) -> bool {
        let stats = self.stats.read().await;
        stats.heap_used_mb > threshold_mb
    }

    /// Get memory usage trend over time
    pub async fn get_trend(&self, window_seconds: u64) -> MemoryTrend {
        let history = self.allocation_history.read().await;
        let now = Instant::now();
        let window = Duration::from_secs(window_seconds);

        let recent: Vec<f64> = history
            .iter()
            .filter(|(t, _)| now.duration_since(*t) <= window)
            .map(|(_, mb)| *mb)
            .collect();

        if recent.is_empty() {
            return MemoryTrend::Stable;
        }

        let first = recent.first().unwrap();
        let last = recent.last().unwrap();

        let change = (last - first) / first;

        if change > 0.1 {
            MemoryTrend::Increasing
        } else if change < -0.1 {
            MemoryTrend::Decreasing
        } else {
            MemoryTrend::Stable
        }
    }

    /// Suggest memory optimization strategies
    pub async fn suggest_optimizations(&self) -> Vec<String> {
        let mut suggestions = Vec::new();
        let stats = self.stats.read().await;

        if stats.heap_used_mb > 1000.0 {
            suggestions.push("Consider clearing caches to free memory".to_string());
        }

        if stats.heap_peak_mb > 2000.0 {
            suggestions
                .push("Peak memory usage is high - consider reducing batch sizes".to_string());
        }

        let trend = self.get_trend(60).await;
        if matches!(trend, MemoryTrend::Increasing) {
            suggestions.push("Memory usage is increasing - check for memory leaks".to_string());
        }

        suggestions
    }
}

/// Memory usage trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrend {
    Increasing,
    Decreasing,
    Stable,
}

/// Estimate memory usage for a data structure
pub fn estimate_memory_size<T>(count: usize) -> f64 {
    // Rough estimation: size of T * count
    let size_per_item = std::mem::size_of::<T>();
    (size_per_item * count) as f64 / (1024.0 * 1024.0) // Convert to MB
}

/// Memory optimization utilities
pub struct MemoryOptimizer;

impl MemoryOptimizer {
    /// Force garbage collection (if available)
    pub fn suggest_gc() {
        // In Rust, we can't force GC, but we can suggest dropping references
        debug!("Suggesting memory cleanup - drop unused references");
    }

    /// Clear caches if memory pressure is high
    pub async fn clear_caches_if_needed(
        profiler: &MemoryProfiler,
        caches: &[Box<dyn CacheClearable>],
        threshold_mb: f64,
    ) {
        if profiler.check_memory_pressure(threshold_mb).await {
            warn!(
                "Memory pressure detected (>{:.2} MB), clearing caches",
                threshold_mb
            );
            for cache in caches {
                cache.clear().await;
            }
        }
    }
}

/// Trait for caches that can be cleared
#[async_trait::async_trait]
pub trait CacheClearable: Send + Sync {
    async fn clear(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_profiler() {
        let profiler = MemoryProfiler::new(true);

        profiler.record_usage(100.0).await;
        let stats = profiler.get_stats().await;
        assert_eq!(stats.heap_used_mb, 100.0);

        profiler.record_usage(200.0).await;
        let peak = profiler.get_peak_memory().await;
        assert_eq!(peak, 200.0);
    }

    #[tokio::test]
    async fn test_memory_pressure() {
        let profiler = MemoryProfiler::new(true);
        profiler.record_usage(1500.0).await;

        assert!(profiler.check_memory_pressure(1000.0).await);
        assert!(!profiler.check_memory_pressure(2000.0).await);
    }
}
