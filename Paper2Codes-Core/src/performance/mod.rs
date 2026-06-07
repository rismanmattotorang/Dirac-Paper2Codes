//! Performance optimization system
//!
//! Includes adaptive concurrency control, smart rate limiting, memory profiling, and caching.

pub mod cache;
pub mod concurrency;
pub mod memory;
pub mod rate_limiter;

pub use cache::{CacheConfig, MetadataCache};
pub use concurrency::{AdaptiveConcurrency, ConcurrencyConfig};
pub use memory::{MemoryOptimizer, MemoryProfiler, MemoryStats, MemoryTrend};
pub use rate_limiter::{RateLimitConfig, RateLimitStats, SmartRateLimiter};
