/// Embedding service for generating and managing text embeddings
/// Supports multiple embedding providers for state-of-the-art RAG
use crate::error::{LLMError, Paper2CodesError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Embedding provider trait for flexibility
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch processing)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;

    /// Get provider name
    fn provider_name(&self) -> &str;
}

/// OpenAI embedding provider using text-embedding-3-large (state-of-the-art)
pub struct OpenAIEmbeddingProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
    dimension: usize,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, "text-embedding-3-large".to_string())
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        let dimension = if model.contains("large") { 3072 } else { 1536 };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            client,
            dimension,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.embed_batch(&[text.to_string()]).await?;
        result
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::InvalidResponse("No embedding returned".to_string()).into())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            input: Vec<String>,
            encoding_format: String,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }

        let request = Request {
            model: self.model.clone(),
            input: texts.to_vec(),
            encoding_format: "float".to_string(),
        };

        // Retry logic with exponential backoff
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            let response = match self
                .client
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(LLMError::Network(e));
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_millis(100 * (1 << attempt));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(last_error.unwrap().into());
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();

                // Retry on rate limit or server errors
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    last_error = Some(LLMError::RequestFailed(format!(
                        "HTTP {}: {}",
                        status, text
                    )));
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_millis(1000 * (1 << attempt));
                        tracing::warn!(
                            "Retry attempt {}/{} after {}ms",
                            attempt + 1,
                            MAX_RETRIES,
                            delay.as_millis()
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(last_error.unwrap().into());
                }

                return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
            }

            let response_body: Response = response.json().await.map_err(|e| {
                LLMError::InvalidResponse(format!("Failed to parse response: {}", e))
            })?;

            // Sort by index to ensure correct order
            let mut embeddings: Vec<(usize, Vec<f32>)> = response_body
                .data
                .into_iter()
                .map(|d| (d.index, d.embedding))
                .collect();
            embeddings.sort_by_key(|(idx, _)| *idx);

            return Ok(embeddings.into_iter().map(|(_, emb)| emb).collect());
        }

        Err(last_error.unwrap_or(LLMError::Timeout).into())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

/// Voyage AI embedding provider (specialized for retrieval)
pub struct VoyageEmbeddingProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
    dimension: usize,
}

impl VoyageEmbeddingProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, "voyage-large-2".to_string())
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        let dimension = 1536; // Voyage large-2 dimension

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            model,
            client,
            dimension,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for VoyageEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.embed_batch(&[text.to_string()]).await?;
        result
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::InvalidResponse("No embedding returned".to_string()).into())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct Request {
            model: String,
            input: Vec<String>,
            input_type: String,
        }

        #[derive(Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let request = Request {
            model: self.model.clone(),
            input: texts.to_vec(),
            input_type: "document".to_string(),
        };

        // Retry logic with exponential backoff (consistent with OpenAI provider)
        const MAX_RETRIES: u32 = 3;
        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            let response = match self
                .client
                .post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(LLMError::Network(e));
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_millis(100 * (1 << attempt));
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(last_error.unwrap().into());
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();

                // Retry on rate limit or server errors
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    last_error = Some(LLMError::RequestFailed(format!(
                        "HTTP {}: {}",
                        status, text
                    )));
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_millis(1000 * (1 << attempt));
                        tracing::warn!(
                            "Voyage AI retry attempt {}/{} after {}ms",
                            attempt + 1,
                            MAX_RETRIES,
                            delay.as_millis()
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(last_error.unwrap().into());
                }

                return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
            }

            let response_body: Response = response.json().await.map_err(|e| {
                LLMError::InvalidResponse(format!("Failed to parse response: {}", e))
            })?;

            return Ok(response_body
                .data
                .into_iter()
                .map(|d| d.embedding)
                .collect());
        }

        Err(last_error.unwrap_or(LLMError::Timeout).into())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn provider_name(&self) -> &str {
        "voyage"
    }
}

/// Embedding service with caching and batching
pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    batch_size: usize,
}

impl EmbeddingService {
    pub fn new(provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            provider,
            cache: Arc::new(RwLock::new(HashMap::new())),
            batch_size: 64, // Optimal batch size for most providers
        }
    }

    /// Create a new EmbeddingService with OpenAI provider
    pub fn new_openai(api_key: String) -> Result<Self> {
        let provider = Arc::new(OpenAIEmbeddingProvider::new(api_key));
        Ok(Self::new(provider))
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }

    /// Generate embedding with caching
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Generate embedding
        let embedding = self.provider.embed(text).await?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(text.to_string(), embedding.clone());
        }

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts with batching and caching
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut cache_misses = Vec::new();
        let mut cache_miss_indices = Vec::new();

        // Check cache
        {
            let cache = self.cache.read().await;
            for (idx, text) in texts.iter().enumerate() {
                if let Some(embedding) = cache.get(text) {
                    results.push((idx, embedding.clone()));
                } else {
                    cache_misses.push(text.clone());
                    cache_miss_indices.push(idx);
                }
            }
        }

        // Generate embeddings for cache misses in batches
        if !cache_misses.is_empty() {
            let mut new_embeddings = Vec::new();

            for chunk in cache_misses.chunks(self.batch_size) {
                let embeddings = self.provider.embed_batch(chunk).await?;
                new_embeddings.extend(embeddings);
            }

            // Update cache and results
            {
                let mut cache = self.cache.write().await;
                for (text, embedding) in cache_misses.iter().zip(new_embeddings.iter()) {
                    cache.insert(text.clone(), embedding.clone());
                }
            }

            // Add new embeddings to results
            for (idx, embedding) in cache_miss_indices.into_iter().zip(new_embeddings) {
                results.push((idx, embedding));
            }
        }

        // Sort by original index and extract embeddings
        results.sort_by_key(|(idx, _)| *idx);
        Ok(results.into_iter().map(|(_, emb)| emb).collect())
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Save cache to disk (for persistence)
    pub async fn save_cache(&self, path: &std::path::Path) -> Result<()> {
        let cache = self.cache.read().await;
        let serialized =
            serde_json::to_string(&*cache).map_err(|e| Paper2CodesError::Serialization(e))?;

        tokio::fs::write(path, serialized)
            .await
            .map_err(|e| Paper2CodesError::Io(e))?;

        Ok(())
    }

    /// Load cache from disk
    pub async fn load_cache(&self, path: &std::path::Path) -> Result<()> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Paper2CodesError::Io(e))?;

        let loaded: HashMap<String, Vec<f32>> =
            serde_json::from_str(&content).map_err(|e| Paper2CodesError::Serialization(e))?;

        let mut cache = self.cache.write().await;
        *cache = loaded;

        Ok(())
    }
}

/// Create embedding service from configuration
pub fn create_embedding_service(provider_name: &str, api_key: String) -> Result<EmbeddingService> {
    let provider: Arc<dyn EmbeddingProvider> = match provider_name.to_lowercase().as_str() {
        "openai" => Arc::new(OpenAIEmbeddingProvider::new(api_key)),
        "voyage" => Arc::new(VoyageEmbeddingProvider::new(api_key)),
        _ => {
            return Err(Paper2CodesError::Config(
                crate::error::ConfigError::Invalid(format!(
                    "Unknown embedding provider: {}",
                    provider_name
                )),
            ))
        }
    };

    Ok(EmbeddingService::new(provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock provider for testing
    struct MockEmbeddingProvider;

    #[async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        async fn embed(&self, text: &str) -> Result<Vec<f32>> {
            // Simple mock: return hash-based embedding
            let hash = text.len() as f32;
            Ok(vec![hash, hash * 2.0, hash * 3.0])
        }

        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            let mut results = Vec::new();
            for text in texts {
                results.push(self.embed(text).await?);
            }
            Ok(results)
        }

        fn dimension(&self) -> usize {
            3
        }

        fn provider_name(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_embedding_service_caching() {
        let provider = Arc::new(MockEmbeddingProvider);
        let service = EmbeddingService::new(provider);

        // First call should generate embedding
        let embedding1 = service.embed("test text").await.unwrap();
        assert_eq!(embedding1.len(), 3);
        assert_eq!(service.cache_size().await, 1);

        // Second call should use cache
        let embedding2 = service.embed("test text").await.unwrap();
        assert_eq!(embedding1, embedding2);
        assert_eq!(service.cache_size().await, 1);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let provider = Arc::new(MockEmbeddingProvider);
        let service = EmbeddingService::new(provider);

        let texts = vec![
            "text1".to_string(),
            "text2".to_string(),
            "text3".to_string(),
        ];
        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        assert_eq!(service.cache_size().await, 3);
    }
}
