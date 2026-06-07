//! Advanced caching with metadata and TTL support
use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// Configuration for caching
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub capacity: usize,
    pub max_age_secs: u64,
    pub max_access_count: u32,
    pub cleanup_interval_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            max_age_secs: 3600,
            max_access_count: 100,
            cleanup_interval_secs: 300, // 5 minutes
        }
    }
}

struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u32,
    ttl: Option<Duration>,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl,
        }
    }

    fn update_access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        // Check TTL first if set
        if let Some(ttl) = self.ttl {
            if self.created_at.elapsed() > ttl {
                return true;
            }
        }

        // Check max age
        if self.last_accessed.elapsed() > max_age {
            return true;
        }

        false
    }
}

/// Metadata-aware LRU cache with TTL support
pub struct MetadataCache<K, V> {
    cache: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    max_age: Duration,
    max_access_count: u32,
    cleanup_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl<K, V> MetadataCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(config: CacheConfig) -> Self {
        let cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(config.capacity).unwrap(),
        )));

        let max_age = Duration::from_secs(config.max_age_secs);
        let max_access_count = config.max_access_count;
        let cleanup_interval = Duration::from_secs(config.cleanup_interval_secs);

        // Start background cleanup task
        let cache_clone = Arc::clone(&cache);
        let max_age_clone = max_age;
        let max_access_count_clone = max_access_count;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                Self::cleanup_expired(&cache_clone, max_age_clone, max_access_count_clone).await;
            }
        });

        Self {
            cache,
            max_age,
            max_access_count,
            cleanup_handle: Arc::new(RwLock::new(Some(handle))),
        }
    }

    /// Cleanup expired entries
    async fn cleanup_expired(
        cache: &Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
        max_age: Duration,
        max_access_count: u32,
    ) {
        let mut cache_guard = cache.write().await;
        let mut count = 0;

        // Iterate and remove expired entries in a single pass
        // Use a vector to collect keys since we can't mutate while iterating
        let mut keys_to_remove = Vec::new();
        for (key, entry) in cache_guard.iter() {
            if entry.is_expired(max_age) || entry.access_count >= max_access_count {
                keys_to_remove.push(key.clone());
            }
        }

        // Remove collected keys
        for key in keys_to_remove {
            if cache_guard.pop(&key).is_some() {
                count += 1;
            }
        }

        if count > 0 {
            debug!("Cleaned up {} expired cache entries", count);
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;

        // Check if entry exists and is valid
        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is expired
            if entry.is_expired(self.max_age) || entry.access_count >= self.max_access_count {
                cache.pop(key);
                return None;
            }

            // Update access metadata
            entry.update_access();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, None).await;
    }

    /// Insert with a specific TTL
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut cache = self.cache.write().await;
        cache.put(key, CacheEntry::new(value, ttl));
    }

    pub async fn remove(&self, key: &K) {
        let mut cache = self.cache.write().await;
        cache.pop(key);
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;

        let total_entries = cache.len();
        let expired_count = cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.max_age))
            .count();

        let total_accesses: u32 = cache.iter().map(|(_, entry)| entry.access_count).sum();

        CacheStats {
            total_entries,
            expired_entries: expired_count,
            total_accesses,
            capacity: cache.cap().get(),
        }
    }
}

impl<K, V> Drop for MetadataCache<K, V> {
    fn drop(&mut self) {
        // Abort cleanup task gracefully
        // Use blocking wait to ensure cleanup task is properly terminated
        if let Ok(mut handle) = self.cleanup_handle.try_write() {
            if let Some(h) = handle.take() {
                h.abort();
                // Give the task a moment to clean up
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_accesses: u32,
    pub capacity: usize,
}

impl CacheStats {
    pub fn usage_ratio(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.total_entries as f64 / self.capacity as f64
        }
    }

    pub fn hit_rate_estimate(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.total_accesses as f64 / (self.total_entries as f64 + self.total_accesses as f64)
        }
    }
}
