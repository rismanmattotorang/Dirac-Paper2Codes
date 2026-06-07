/// LLM response caching for performance optimization
/// Reduces API calls and costs for repeated queries
use crate::error::{Paper2CodesError, Result};
use crate::llm::client::{LLMResponse, Message};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache key generated from request parameters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub model: String,
    pub messages_hash: String, // SHA256 hash of messages
    pub temperature: String,   // Rounded to avoid minor differences
}

impl CacheKey {
    pub fn from_request(model: &str, messages: &[Message], temperature: f32) -> Self {
        // Create a deterministic hash of messages (more efficient)
        let mut hasher = Sha256::new();
        for message in messages {
            // Use a more efficient serialization approach
            hasher.update(format!("{:?}", message.role).as_bytes());
            hasher.update(b"\0"); // Separator
            hasher.update(message.content.as_bytes());
            hasher.update(b"\0"); // Separator
        }
        let messages_hash = format!("{:x}", hasher.finalize());

        // Round temperature to 2 decimal places for cache key stability
        let temperature_str = format!("{:.2}", temperature);

        Self {
            model: model.to_string(),
            messages_hash,
            temperature: temperature_str,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub response: LLMResponse,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub hit_count: usize,
}

/// LLM response cache with configurable TTL and size limits
pub struct LLMCache {
    cache: Arc<RwLock<HashMap<CacheKey, CachedResponse>>>,
    max_entries: usize,
    ttl_seconds: u64,
}

impl LLMCache {
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            ttl_seconds,
        }
    }

    /// Get response from cache if available and not expired
    pub async fn get(&self, key: &CacheKey) -> Option<LLMResponse> {
        let mut cache = self.cache.write().await;

        if let Some(cached) = cache.get_mut(key) {
            // Check if expired
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(cached.timestamp);

            if age.num_seconds() > self.ttl_seconds as i64 {
                // Expired, remove from cache
                cache.remove(key);
                return None;
            }

            // Update hit count and return
            cached.hit_count += 1;
            Some(cached.response.clone())
        } else {
            None
        }
    }

    /// Store response in cache
    pub async fn put(&self, key: CacheKey, response: LLMResponse) {
        let mut cache = self.cache.write().await;

        // Check size limit and evict if necessary
        if cache.len() >= self.max_entries {
            self.evict_oldest(&mut cache);
        }

        let cached = CachedResponse {
            response,
            timestamp: chrono::Utc::now(),
            hit_count: 0,
        };

        cache.insert(key, cached);
    }

    /// Evict oldest entries (LRU-style)
    fn evict_oldest(&self, cache: &mut HashMap<CacheKey, CachedResponse>) {
        // Find entry with oldest timestamp
        if let Some((key, _)) = cache
            .iter()
            .min_by_key(|(_, cached)| cached.timestamp)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            cache.remove(&key);
        }
    }

    /// Clear all cached responses
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;

        let total_entries = cache.len();
        let total_hits: usize = cache.values().map(|c| c.hit_count).sum();

        CacheStats {
            total_entries,
            total_hits,
            max_entries: self.max_entries,
        }
    }

    /// Save cache to disk
    pub async fn save_to_disk(&self, path: &Path) -> Result<()> {
        let cache = self.cache.read().await;

        let serialized =
            serde_json::to_string(&*cache).map_err(|e| Paper2CodesError::Serialization(e))?;

        tokio::fs::write(path, serialized)
            .await
            .map_err(|e| Paper2CodesError::Io(e))?;

        Ok(())
    }

    /// Load cache from disk
    pub async fn load_from_disk(&self, path: &Path) -> Result<()> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Paper2CodesError::Io(e))?;

        let loaded: HashMap<CacheKey, CachedResponse> =
            serde_json::from_str(&content).map_err(|e| Paper2CodesError::Serialization(e))?;

        let mut cache = self.cache.write().await;

        // Only load non-expired entries
        let now = chrono::Utc::now();
        for (key, cached) in loaded {
            let age = now.signed_duration_since(cached.timestamp);
            if age.num_seconds() <= self.ttl_seconds as i64 {
                cache.insert(key, cached);
            }
        }

        Ok(())
    }

    /// Remove expired entries
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = chrono::Utc::now();

        cache.retain(|_, cached| {
            let age = now.signed_duration_since(cached.timestamp);
            age.num_seconds() <= self.ttl_seconds as i64
        });
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: usize,
    pub max_entries: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_hits == 0 {
            0.0
        } else {
            // This is a simplified calculation
            // In practice, you'd track total requests separately
            self.total_hits as f64 / (self.total_entries as f64 + self.total_hits as f64)
        }
    }

    pub fn usage(&self) -> f64 {
        self.total_entries as f64 / self.max_entries as f64
    }
}

impl Default for LLMCache {
    fn default() -> Self {
        Self::new(1000, 3600) // 1000 entries, 1 hour TTL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::MessageRole;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = LLMCache::new(10, 3600);

        let key = CacheKey {
            model: "test-model".to_string(),
            messages_hash: "test-hash".to_string(),
            temperature: "0.70".to_string(),
        };

        let response = LLMResponse {
            content: "Test response".to_string(),
            model: "test-model".to_string(),
            tokens_used: Some(100),
            finish_reason: Some("stop".to_string()),
        };

        // Put and get
        cache.put(key.clone(), response.clone()).await;
        let cached = cache.get(&key).await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Test response");

        // Stats
        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_hits, 1); // One hit from get
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "You are a helpful assistant".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
            },
        ];

        let key1 = CacheKey::from_request("gpt-4", &messages, 0.7);
        let key2 = CacheKey::from_request("gpt-4", &messages, 0.7);
        let key3 = CacheKey::from_request("gpt-4", &messages, 0.8);

        // Same parameters should produce same key
        assert_eq!(key1, key2);

        // Different temperature should produce different key
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_cache_size_limit() {
        let cache = LLMCache::new(3, 3600);

        // Add 4 entries (exceeds limit)
        for i in 0..4 {
            let key = CacheKey {
                model: format!("model-{}", i),
                messages_hash: format!("hash-{}", i),
                temperature: "0.70".to_string(),
            };

            let response = LLMResponse {
                content: format!("Response {}", i),
                model: format!("model-{}", i),
                tokens_used: Some(100),
                finish_reason: Some("stop".to_string()),
            };

            cache.put(key, response).await;

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 3); // Should be limited to 3
    }
}
