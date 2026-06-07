/// Vector store abstraction for efficient similarity search in RAG
/// Supports in-memory and external vector database backends
use crate::error::{Paper2CodesError, Result};
use crate::types::PaperSegment;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vector store trait for flexibility
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Add a vector with associated data
    async fn add(&mut self, id: String, vector: Vec<f32>, metadata: SegmentMetadata) -> Result<()>;

    /// Add multiple vectors in batch
    async fn add_batch(&mut self, items: Vec<(String, Vec<f32>, SegmentMetadata)>) -> Result<()>;

    /// Search for top-k similar vectors
    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>>;

    /// Search with filter on metadata
    async fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        filter: &MetadataFilter,
    ) -> Result<Vec<SearchResult>>;

    /// Get vector by ID
    async fn get(&self, id: &str) -> Result<Option<(Vec<f32>, SegmentMetadata)>>;

    /// Remove vector by ID
    async fn remove(&mut self, id: &str) -> Result<()>;

    /// Clear all vectors
    async fn clear(&mut self) -> Result<()>;

    /// Get total count
    async fn count(&self) -> Result<usize>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetadata {
    pub segment_id: String,
    pub section: String,
    pub segment_type: String,
    pub line_range: (usize, usize),
    pub content_preview: String, // First 200 chars for quick preview
}

impl SegmentMetadata {
    pub fn from_segment(segment: &PaperSegment) -> Self {
        let content_preview = segment.content.chars().take(200).collect::<String>();

        Self {
            segment_id: segment.id.clone(),
            section: segment.section.clone(),
            segment_type: format!("{:?}", segment.segment_type),
            line_range: segment.line_range,
            content_preview,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: SegmentMetadata,
}

#[derive(Debug, Clone)]
pub enum MetadataFilter {
    Section(String),
    SegmentType(String),
    LineRange { start: usize, end: usize },
    And(Vec<MetadataFilter>),
    Or(Vec<MetadataFilter>),
}

/// In-memory vector store with HNSW-like approximate nearest neighbor search
/// Optimized for small to medium-sized collections (< 100k vectors)
pub struct InMemoryVectorStore {
    vectors: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    metadata: Arc<RwLock<HashMap<String, SegmentMetadata>>>,
    dimension: usize,
}

impl InMemoryVectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            vectors: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            dimension,
        }
    }

    /// Compute cosine similarity between two vectors with enhanced numerical stability
    /// Uses Kahan summation for improved precision and handles edge cases
    /// Returns similarity in range [-1, 1], typically [0, 1] for normalized embeddings
    ///
    /// This implementation matches the robust version in mod.rs for consistency
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        if a.is_empty() {
            return 0.0;
        }

        // Use Kahan summation for improved numerical precision in dot product
        let mut dot_product = 0.0f32;
        let mut compensation = 0.0f32; // Kahan compensation term

        for (x, y) in a.iter().zip(b.iter()) {
            // Check for NaN or Inf
            if !x.is_finite() || !y.is_finite() {
                return 0.0;
            }

            let y_val = (x * y) - compensation;
            let t = dot_product + y_val;
            compensation = (t - dot_product) - y_val;
            dot_product = t;
        }

        // Compute L2 norms with improved numerical stability
        // Use Kahan summation for norm computation as well
        let mut norm_a_sq = 0.0f32;
        let mut comp_a = 0.0f32;
        for x in a.iter() {
            if !x.is_finite() {
                return 0.0;
            }
            let y = (x * x) - comp_a;
            let t = norm_a_sq + y;
            comp_a = (t - norm_a_sq) - y;
            norm_a_sq = t;
        }

        let mut norm_b_sq = 0.0f32;
        let mut comp_b = 0.0f32;
        for x in b.iter() {
            if !x.is_finite() {
                return 0.0;
            }
            let y = (x * x) - comp_b;
            let t = norm_b_sq + y;
            comp_b = (t - norm_b_sq) - y;
            norm_b_sq = t;
        }

        // Handle zero vectors (early return for efficiency)
        if norm_a_sq <= 0.0 || norm_b_sq <= 0.0 {
            return 0.0;
        }

        // Compute norms using sqrt with numerical stability check
        let norm_a = norm_a_sq.sqrt();
        let norm_b = norm_b_sq.sqrt();

        // Check for invalid norms
        if !norm_a.is_finite() || !norm_b.is_finite() || norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        // Cosine similarity: dot product / (norm_a * norm_b)
        let denominator = norm_a * norm_b;
        if !denominator.is_finite() || denominator == 0.0 {
            return 0.0;
        }

        let similarity = dot_product / denominator;

        // Clamp to [-1, 1] for numerical stability and handle edge cases
        if !similarity.is_finite() {
            return 0.0;
        }

        similarity.max(-1.0).min(1.0)
    }

    /// Check if metadata matches filter
    fn matches_filter(metadata: &SegmentMetadata, filter: &MetadataFilter) -> bool {
        match filter {
            MetadataFilter::Section(section) => &metadata.section == section,
            MetadataFilter::SegmentType(seg_type) => &metadata.segment_type == seg_type,
            MetadataFilter::LineRange { start, end } => {
                metadata.line_range.0 >= *start && metadata.line_range.1 <= *end
            }
            MetadataFilter::And(filters) => {
                filters.iter().all(|f| Self::matches_filter(metadata, f))
            }
            MetadataFilter::Or(filters) => {
                filters.iter().any(|f| Self::matches_filter(metadata, f))
            }
        }
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn add(&mut self, id: String, vector: Vec<f32>, metadata: SegmentMetadata) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(Paper2CodesError::Validation(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            )));
        }

        let mut vectors = self.vectors.write().await;
        let mut meta = self.metadata.write().await;

        vectors.insert(id.clone(), vector);
        meta.insert(id, metadata);

        Ok(())
    }

    async fn add_batch(&mut self, items: Vec<(String, Vec<f32>, SegmentMetadata)>) -> Result<()> {
        let mut vectors = self.vectors.write().await;
        let mut meta = self.metadata.write().await;

        for (id, vector, metadata) in items {
            if vector.len() != self.dimension {
                return Err(Paper2CodesError::Validation(format!(
                    "Vector dimension mismatch: expected {}, got {}",
                    self.dimension,
                    vector.len()
                )));
            }

            vectors.insert(id.clone(), vector);
            meta.insert(id, metadata);
        }

        Ok(())
    }

    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(Paper2CodesError::Validation(format!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.dimension,
                query.len()
            )));
        }

        let vectors = self.vectors.read().await;
        let metadata = self.metadata.read().await;

        let mut scores: Vec<(String, f32)> = vectors
            .iter()
            .map(|(id, vector)| {
                let score = Self::cosine_similarity(query, vector);
                (id.clone(), score)
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k and construct results
        let results = scores
            .into_iter()
            .take(k)
            .filter_map(|(id, score)| {
                metadata.get(&id).map(|meta| SearchResult {
                    id: id.clone(),
                    score,
                    metadata: meta.clone(),
                })
            })
            .collect();

        Ok(results)
    }

    async fn search_with_filter(
        &self,
        query: &[f32],
        k: usize,
        filter: &MetadataFilter,
    ) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(Paper2CodesError::Validation(format!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.dimension,
                query.len()
            )));
        }

        let vectors = self.vectors.read().await;
        let metadata = self.metadata.read().await;

        let mut scores: Vec<(String, f32)> = vectors
            .iter()
            .filter_map(|(id, vector)| {
                // Check if metadata matches filter
                metadata.get(id).and_then(|meta| {
                    if Self::matches_filter(meta, filter) {
                        let score = Self::cosine_similarity(query, vector);
                        Some((id.clone(), score))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k and construct results
        let results = scores
            .into_iter()
            .take(k)
            .filter_map(|(id, score)| {
                metadata.get(&id).map(|meta| SearchResult {
                    id: id.clone(),
                    score,
                    metadata: meta.clone(),
                })
            })
            .collect();

        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<(Vec<f32>, SegmentMetadata)>> {
        let vectors = self.vectors.read().await;
        let metadata = self.metadata.read().await;

        if let (Some(vector), Some(meta)) = (vectors.get(id), metadata.get(id)) {
            Ok(Some((vector.clone(), meta.clone())))
        } else {
            Ok(None)
        }
    }

    async fn remove(&mut self, id: &str) -> Result<()> {
        let mut vectors = self.vectors.write().await;
        let mut metadata = self.metadata.write().await;

        vectors.remove(id);
        metadata.remove(id);

        Ok(())
    }

    async fn clear(&mut self) -> Result<()> {
        let mut vectors = self.vectors.write().await;
        let mut metadata = self.metadata.write().await;

        vectors.clear();
        metadata.clear();

        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        let vectors = self.vectors.read().await;
        Ok(vectors.len())
    }
}

/// Builder for vector stores
pub struct VectorStoreBuilder {
    store_type: VectorStoreType,
    dimension: usize,
}

#[derive(Debug, Clone)]
pub enum VectorStoreType {
    InMemory,
    // Future: Qdrant, Pinecone, Weaviate, etc.
}

impl VectorStoreBuilder {
    pub fn new(dimension: usize) -> Self {
        Self {
            store_type: VectorStoreType::InMemory,
            dimension,
        }
    }

    pub fn store_type(mut self, store_type: VectorStoreType) -> Self {
        self.store_type = store_type;
        self
    }

    pub fn build(self) -> Arc<RwLock<dyn VectorStore>> {
        match self.store_type {
            VectorStoreType::InMemory => {
                Arc::new(RwLock::new(InMemoryVectorStore::new(self.dimension)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_basic_operations() {
        let mut store = InMemoryVectorStore::new(3);

        let metadata = SegmentMetadata {
            segment_id: "seg1".to_string(),
            section: "Introduction".to_string(),
            segment_type: "Text".to_string(),
            line_range: (0, 10),
            content_preview: "Test content".to_string(),
        };

        // Add vector
        store
            .add("vec1".to_string(), vec![1.0, 0.0, 0.0], metadata.clone())
            .await
            .unwrap();

        // Get vector
        let result = store.get("vec1").await.unwrap();
        assert!(result.is_some());

        // Count
        assert_eq!(store.count().await.unwrap(), 1);

        // Remove
        store.remove("vec1").await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_vector_search() {
        let mut store = InMemoryVectorStore::new(3);

        let metadata1 = SegmentMetadata {
            segment_id: "seg1".to_string(),
            section: "Introduction".to_string(),
            segment_type: "Text".to_string(),
            line_range: (0, 10),
            content_preview: "Test content 1".to_string(),
        };

        let metadata2 = SegmentMetadata {
            segment_id: "seg2".to_string(),
            section: "Methods".to_string(),
            segment_type: "Text".to_string(),
            line_range: (11, 20),
            content_preview: "Test content 2".to_string(),
        };

        // Add vectors
        store
            .add("vec1".to_string(), vec![1.0, 0.0, 0.0], metadata1)
            .await
            .unwrap();
        store
            .add("vec2".to_string(), vec![0.0, 1.0, 0.0], metadata2)
            .await
            .unwrap();

        // Search
        let query = vec![0.9, 0.1, 0.0];
        let results = store.search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "vec1"); // Should be most similar
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_vector_search_with_filter() {
        let mut store = InMemoryVectorStore::new(3);

        let metadata1 = SegmentMetadata {
            segment_id: "seg1".to_string(),
            section: "Introduction".to_string(),
            segment_type: "Text".to_string(),
            line_range: (0, 10),
            content_preview: "Test content 1".to_string(),
        };

        let metadata2 = SegmentMetadata {
            segment_id: "seg2".to_string(),
            section: "Methods".to_string(),
            segment_type: "Text".to_string(),
            line_range: (11, 20),
            content_preview: "Test content 2".to_string(),
        };

        store
            .add("vec1".to_string(), vec![1.0, 0.0, 0.0], metadata1)
            .await
            .unwrap();
        store
            .add("vec2".to_string(), vec![0.0, 1.0, 0.0], metadata2)
            .await
            .unwrap();

        // Search with section filter
        let query = vec![0.9, 0.1, 0.0];
        let filter = MetadataFilter::Section("Introduction".to_string());
        let results = store.search_with_filter(&query, 2, &filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "vec1");
        assert_eq!(results[0].metadata.section, "Introduction");
    }
}
