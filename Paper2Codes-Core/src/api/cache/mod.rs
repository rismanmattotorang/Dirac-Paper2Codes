//! API Response Caching System
//!
//! This module provides a comprehensive caching system for API responses with:
//! - In-memory LRU cache for hot data
//! - Optional Redis integration for distributed caching
//! - HTTP cache headers support
//! - Cache invalidation strategies
//! - Cache warming capabilities

pub mod in_memory;
pub mod invalidation;
pub mod keys;
pub mod middleware;

pub use in_memory::InMemoryCache;
pub use invalidation::CacheInvalidator;
pub use keys::CacheKey;
pub use middleware::cache_middleware;

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    /// Default TTL for cached entries (seconds)
    #[serde(default = "default_cache_ttl")]
    pub default_ttl: u64,

    /// Maximum number of entries in in-memory cache
    #[serde(default = "default_cache_size")]
    pub max_entries: usize,

    /// Enable Redis for distributed caching
    #[serde(default = "default_redis_enabled")]
    pub redis_enabled: bool,

    /// Redis connection URL (if enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis_url: Option<String>,

    /// Cache warming enabled
    #[serde(default = "default_cache_warming")]
    pub enable_warming: bool,
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    300 // 5 minutes
}

fn default_cache_size() -> usize {
    1000
}

fn default_redis_enabled() -> bool {
    false
}

fn default_cache_warming() -> bool {
    false
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            default_ttl: default_cache_ttl(),
            max_entries: default_cache_size(),
            redis_enabled: default_redis_enabled(),
            redis_url: None,
            enable_warming: default_cache_warming(),
        }
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: Vec<u8>,
    pub content_type: String,
    pub created_at: std::time::Instant,
    pub ttl: Duration,
    pub access_count: u64,
    pub last_accessed: std::time::Instant,
}

impl CacheEntry {
    pub fn new(data: Vec<u8>, content_type: String, ttl: Duration) -> Self {
        let now = std::time::Instant::now();
        Self {
            data,
            content_type,
            created_at: now,
            ttl,
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = std::time::Instant::now();
    }

    pub fn remaining_ttl(&self) -> Duration {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.ttl {
            Duration::ZERO
        } else {
            self.ttl - elapsed
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub memory_usage_bytes: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}
