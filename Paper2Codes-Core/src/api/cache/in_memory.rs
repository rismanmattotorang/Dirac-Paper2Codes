//! In-memory LRU cache implementation
//!
//! Provides a high-performance in-memory cache with LRU eviction policy

use super::{CacheEntry, CacheStats};
use crate::api::cache::keys::CacheKey;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::debug;

/// In-memory LRU cache for API responses
pub struct InMemoryCache {
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    default_ttl: Duration,
}

impl InMemoryCache {
    /// Create a new in-memory cache
    pub fn new(max_entries: usize, default_ttl: Duration) -> Self {
        let capacity =
            NonZeroUsize::new(max_entries.max(1)).expect("Cache capacity must be at least 1");

        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            default_ttl,
        }
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        let key_str = key.to_string();
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get(&key_str) {
            if entry.is_expired() {
                // Remove expired entry
                cache.pop(&key_str);
                stats.misses += 1;
                debug!(key = %key, "Cache miss: entry expired");
                return None;
            }

            // Record access
            let mut entry = entry.clone();
            entry.record_access();
            cache.put(key_str.clone(), entry);

            stats.hits += 1;
            debug!(key = %key, "Cache hit");

            // Return a clone of the data
            cache.get(&key_str).map(|e| e.data.clone())
        } else {
            stats.misses += 1;
            debug!(key = %key, "Cache miss: not found");
            None
        }
    }

    /// Store a value in the cache
    pub async fn set(
        &self,
        key: &CacheKey,
        data: Vec<u8>,
        content_type: String,
        ttl: Option<Duration>,
    ) -> bool {
        let key_str = key.to_string();
        let ttl = ttl.unwrap_or(self.default_ttl);
        let entry = CacheEntry::new(data, content_type, ttl);

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        // Check if we need to evict
        if cache.len() >= cache.cap().get() && !cache.contains(&key_str) {
            if let Some((evicted_key, _)) = cache.pop_lru() {
                stats.evictions += 1;
                debug!(key = %evicted_key, "Cache eviction");
            }
        }

        cache.put(key_str.clone(), entry);
        stats.entries = cache.len();

        debug!(key = %key, ttl_secs = ttl.as_secs(), "Cache set");
        true
    }

    /// Remove a value from the cache
    pub async fn invalidate(&self, key: &CacheKey) -> bool {
        let key_str = key.to_string();
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        let removed = cache.pop(&key_str).is_some();
        if removed {
            stats.entries = cache.len();
            debug!(key = %key, "Cache invalidated");
        }

        removed
    }

    /// Invalidate all entries matching a pattern
    pub async fn invalidate_pattern(&self, pattern: &str) -> usize {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(key, _)| key.contains(pattern))
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            cache.pop(&key);
        }

        stats.entries = cache.len();
        debug!(
            pattern = pattern,
            count = count,
            "Cache invalidated by pattern"
        );

        count
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        cache.clear();
        stats.entries = 0;

        debug!("Cache cleared");
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let cache = self.cache.read().await;

        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            evictions: stats.evictions,
            entries: cache.len(),
            memory_usage_bytes: self.estimate_memory_usage(&cache).await,
        }
    }

    /// Estimate memory usage of the cache
    async fn estimate_memory_usage(&self, cache: &LruCache<String, CacheEntry>) -> usize {
        let mut total = 0;
        for (key, entry) in cache.iter() {
            total += key.len();
            total += entry.data.len();
            total += entry.content_type.len();
            total += std::mem::size_of::<CacheEntry>();
        }
        total
    }

    /// Get the number of entries in the cache
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if the cache is empty
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_get_set() {
        let cache = InMemoryCache::new(10, Duration::from_secs(60));
        let key = CacheKey::paper("123");
        let data = b"test data".to_vec();

        // Set value
        cache
            .set(&key, data.clone(), "application/json".to_string(), None)
            .await;

        // Get value
        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = InMemoryCache::new(10, Duration::from_millis(100));
        let key = CacheKey::paper("123");
        let data = b"test data".to_vec();

        // Set value with short TTL
        cache
            .set(
                &key,
                data.clone(),
                "application/json".to_string(),
                Some(Duration::from_millis(50)),
            )
            .await;

        // Should be available immediately
        assert!(cache.get(&key).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should be expired
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = InMemoryCache::new(10, Duration::from_secs(60));
        let key = CacheKey::paper("123");
        let data = b"test data".to_vec();

        cache
            .set(&key, data, "application/json".to_string(), None)
            .await;
        assert!(cache.get(&key).await.is_some());

        cache.invalidate(&key).await;
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let cache = InMemoryCache::new(2, Duration::from_secs(60));

        // Fill cache to capacity
        cache
            .set(
                &CacheKey::paper("1"),
                b"data1".to_vec(),
                "application/json".to_string(),
                None,
            )
            .await;
        cache
            .set(
                &CacheKey::paper("2"),
                b"data2".to_vec(),
                "application/json".to_string(),
                None,
            )
            .await;

        // Add one more - should evict least recently used
        cache
            .set(
                &CacheKey::paper("3"),
                b"data3".to_vec(),
                "application/json".to_string(),
                None,
            )
            .await;

        // First entry should be evicted
        assert!(cache.get(&CacheKey::paper("1")).await.is_none());
        // But others should still be there
        assert!(cache.get(&CacheKey::paper("2")).await.is_some());
        assert!(cache.get(&CacheKey::paper("3")).await.is_some());
    }
}
