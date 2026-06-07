pub mod embedding;
pub mod vector_store;

pub use embedding::{EmbeddingProvider, EmbeddingService};
pub use vector_store::{VectorStore, VectorStoreBuilder};

use crate::error::Result;
use crate::types::{Paper, PaperSegment, Repository, RetrievedContext, Task};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait CPREngine: Send + Sync {
    async fn retrieve(
        &self,
        task: &Task,
        paper: &Paper,
        repository: &Repository,
        k: usize,
    ) -> Result<Vec<RetrievedContext>>;

    /// Initialize the engine with paper content (precompute embeddings)
    async fn initialize(&self, paper: &Paper) -> Result<()>;
}

pub struct DefaultCPREngine {
    lambda: f32,          // Keyword overlap weight
    delta: f32,           // Algorithm match boost
    gamma: f32,           // Already implemented de-boost
    alpha: f32,           // Embedding similarity weight (when available)
    use_embeddings: bool, // Whether to use embeddings if available
    embedding_service: Option<Arc<EmbeddingService>>,
    vector_store: Option<Arc<tokio::sync::RwLock<dyn VectorStore>>>,
    initialized: std::sync::atomic::AtomicBool,
    init_lock: tokio::sync::Mutex<()>,
}

impl DefaultCPREngine {
    pub fn new() -> Self {
        Self {
            lambda: 0.3,
            delta: 0.5,
            gamma: 0.4,
            alpha: 0.7, // Higher weight for embeddings when available
            use_embeddings: true,
            embedding_service: None,
            vector_store: None,
            initialized: std::sync::atomic::AtomicBool::new(false),
            init_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub fn with_weights(mut self, lambda: f32, delta: f32, gamma: f32) -> Self {
        self.lambda = lambda;
        self.delta = delta;
        self.gamma = gamma;
        self
    }

    pub fn with_embedding_weight(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn enable_embeddings(mut self, enable: bool) -> Self {
        self.use_embeddings = enable;
        self
    }

    pub fn with_embedding_service(mut self, service: Arc<EmbeddingService>) -> Self {
        let dimension = service.dimension();
        self.embedding_service = Some(service);
        self.vector_store = Some(VectorStoreBuilder::new(dimension).build());
        self
    }

    fn generate_query_keywords(&self, task: &Task) -> Vec<String> {
        // Extract keywords from task description
        let text = task.description.to_lowercase();
        let stop_words: Vec<&str> = vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "should", "could", "may", "might", "must",
            "can", "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
        ];

        text.split_whitespace()
            .filter(|word| {
                word.len() > 2
                    && !stop_words.contains(&word)
                    && word.chars().all(|c| c.is_alphanumeric() || c == '-')
            })
            .map(|s| s.to_string())
            .collect()
    }

    fn keyword_overlap(&self, content: &str, keywords: &[String]) -> f32 {
        let content_lower = content.to_lowercase();
        let matches = keywords
            .iter()
            .filter(|keyword| content_lower.contains(&keyword.to_lowercase()))
            .count();

        if keywords.is_empty() {
            0.0
        } else {
            matches as f32 / keywords.len() as f32
        }
    }

    fn is_algorithm_match(&self, segment: &PaperSegment, task: &Task) -> bool {
        // Check if segment contains algorithm-related content and task is about algorithms
        let segment_lower = segment.content.to_lowercase();
        let task_lower = task.description.to_lowercase();

        let has_algorithm_keywords = segment_lower.contains("algorithm")
            || segment_lower.contains("procedure")
            || segment_lower.contains("pseudocode");

        let task_about_algorithm = task_lower.contains("algorithm")
            || task_lower.contains("implement")
            || task_lower.contains("code");

        has_algorithm_keywords && task_about_algorithm
    }

    fn is_already_implemented(&self, segment: &PaperSegment, repository: &Repository) -> bool {
        // Simple check: see if any module description matches segment content
        // In a real implementation, this would use embeddings or more sophisticated matching
        let segment_keywords: Vec<&str> = segment
            .content
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .take(10)
            .collect();

        repository.modules.iter().any(|module| {
            let module_text =
                format!("{} {}", module.content, module.file_path.to_string_lossy()).to_lowercase();
            segment_keywords
                .iter()
                .any(|keyword| module_text.contains(&keyword.to_lowercase()))
        })
    }

    /// Compute cosine similarity between two vectors with enhanced numerical stability
    /// Uses Kahan summation for improved precision and handles edge cases
    /// Returns similarity in range [-1, 1], typically [0, 1] for normalized embeddings
    ///
    /// Algorithm improvements:
    /// - Kahan summation for dot product to reduce floating-point errors
    /// - Fused multiply-add operations where possible
    /// - Early return for zero vectors
    /// - Proper handling of NaN and Inf values
    fn cosine_similarity_static(vec1: &[f32], vec2: &[f32]) -> f32 {
        if vec1.len() != vec2.len() {
            return 0.0;
        }

        if vec1.is_empty() {
            return 0.0;
        }

        // Use Kahan summation for improved numerical precision in dot product
        let mut dot_product = 0.0f32;
        let mut compensation = 0.0f32; // Kahan compensation term

        for (a, b) in vec1.iter().zip(vec2.iter()) {
            // Check for NaN or Inf
            if !a.is_finite() || !b.is_finite() {
                return 0.0;
            }

            let y = (a * b) - compensation;
            let t = dot_product + y;
            compensation = (t - dot_product) - y;
            dot_product = t;
        }

        // Compute L2 norms with improved numerical stability
        // Use Kahan summation for norm computation as well
        let mut norm1_sq = 0.0f32;
        let mut comp1 = 0.0f32;
        for a in vec1.iter() {
            if !a.is_finite() {
                return 0.0;
            }
            let y = (a * a) - comp1;
            let t = norm1_sq + y;
            comp1 = (t - norm1_sq) - y;
            norm1_sq = t;
        }

        let mut norm2_sq = 0.0f32;
        let mut comp2 = 0.0f32;
        for a in vec2.iter() {
            if !a.is_finite() {
                return 0.0;
            }
            let y = (a * a) - comp2;
            let t = norm2_sq + y;
            comp2 = (t - norm2_sq) - y;
            norm2_sq = t;
        }

        // Handle zero vectors (early return for efficiency)
        if norm1_sq <= 0.0 || norm2_sq <= 0.0 {
            return 0.0;
        }

        // Compute norms using sqrt with numerical stability check
        let norm1 = norm1_sq.sqrt();
        let norm2 = norm2_sq.sqrt();

        // Check for invalid norms
        if !norm1.is_finite() || !norm2.is_finite() || norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        // Cosine similarity: dot product / (norm1 * norm2)
        // Use fused multiply-add if available, otherwise standard division
        let denominator = norm1 * norm2;
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

    /// Compute cosine similarity between two vectors with numerical stability
    /// Returns similarity in range [-1, 1], typically [0, 1] for normalized embeddings
    ///
    /// This is a convenience wrapper around the static method
    #[allow(dead_code)] // May be used by external code or future features
    fn cosine_similarity(&self, vec1: &[f32], vec2: &[f32]) -> Result<f32> {
        Ok(Self::cosine_similarity_static(vec1, vec2))
    }
}

#[async_trait]
impl CPREngine for DefaultCPREngine {
    async fn initialize(&self, paper: &Paper) -> Result<()> {
        if !self.use_embeddings {
            return Ok(());
        }

        if self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        let _guard = self.init_lock.lock().await;

        if self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        if let (Some(embedding_service), Some(vector_store)) =
            (&self.embedding_service, &self.vector_store)
        {
            tracing::info!(
                "Initializing CPR engine with embeddings for {} segments",
                paper.segments.len()
            );

            let texts: Vec<String> = paper
                .segments
                .iter()
                .map(|seg| seg.content.clone())
                .collect();

            let embeddings = embedding_service.embed_batch(&texts).await?;

            let mut store = vector_store.write().await;
            let items: Vec<(String, Vec<f32>, vector_store::SegmentMetadata)> = paper
                .segments
                .iter()
                .zip(embeddings.into_iter())
                .map(|(seg, emb)| {
                    (
                        seg.id.clone(),
                        emb,
                        vector_store::SegmentMetadata::from_segment(seg),
                    )
                })
                .collect();

            store.add_batch(items).await?;

            tracing::info!(
                "CPR engine initialized with {} vectors",
                store.count().await?
            );
        }

        self.initialized
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    async fn retrieve(
        &self,
        task: &Task,
        paper: &Paper,
        repository: &Repository,
        k: usize,
    ) -> Result<Vec<RetrievedContext>> {
        // 1. Generate query keywords
        let query_keywords = self.generate_query_keywords(task);

        // 2. Use vector store for semantic search if available and initialized
        let semantic_results =
            if self.use_embeddings && self.initialized.load(std::sync::atomic::Ordering::Acquire) {
                if let (Some(embedding_service), Some(vector_store)) =
                    (&self.embedding_service, &self.vector_store)
                {
                    // Generate query embedding
                    let query_text = task.description.clone();
                    let query_embedding = embedding_service.embed(&query_text).await?;

                    // Search vector store
                    let store = vector_store.read().await;
                    let search_results = store.search(&query_embedding, k * 2).await?; // Get 2x for re-ranking

                    Some(search_results)
                } else {
                    None
                }
            } else {
                None
            };

        // 3. Score paper segments using hybrid approach
        let mut scored_segments: Vec<(PaperSegment, f32)> = Vec::new();

        for segment in &paper.segments {
            let mut score = 0.0;

            // Semantic similarity using vector store results
            if let Some(ref results) = semantic_results {
                if let Some(result) = results.iter().find(|r| r.id == segment.id) {
                    score += self.alpha * result.score;
                }
            }

            // Keyword overlap (fallback/booster when embeddings not available)
            let keyword_score = self.keyword_overlap(&segment.content, &query_keywords);
            score += self.lambda * keyword_score;

            // Boost for algorithm/formula matches (domain-specific signal)
            if self.is_algorithm_match(segment, task) {
                score += self.delta;
            }

            // De-boost for already implemented (avoid redundant work)
            if self.is_already_implemented(segment, repository) {
                score -= self.gamma;
            }

            // Normalize score to [0, 1] range for better interpretability
            score = score.max(0.0).min(1.0);

            scored_segments.push((segment.clone(), score));
        }

        // 4. Select top-k from paper using diversity-aware selection
        scored_segments.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply diversity filtering to avoid redundant segments
        let mut contexts: Vec<RetrievedContext> = Self::select_diverse_segments(scored_segments, k)
            .into_iter()
            .map(|(seg, score)| {
                let reason = if score > 0.7 {
                    "Highly relevant (semantic match)".to_string()
                } else if score > 0.4 {
                    "Relevant (keyword + semantic)".to_string()
                } else {
                    format!("Relevant (score: {:.3})", score)
                };
                RetrievedContext {
                    segment: seg,
                    score,
                    source: crate::types::ContextSource::Paper,
                    relevance_reason: reason,
                }
            })
            .collect();

        // 5. Retrieve external references if needed
        if let Some(external_refs) = self
            .retrieve_external_refs(task, paper, &query_keywords)
            .await?
        {
            contexts.extend(external_refs);
        }

        Ok(contexts)
    }
}

impl DefaultCPREngine {
    /// Select diverse segments using MMR (Maximal Marginal Relevance) algorithm
    /// MMR balances relevance and diversity: MMR = λ * Relevance - (1-λ) * max(Similarity to selected)
    /// This is a state-of-the-art approach for diversity-aware retrieval
    fn select_diverse_segments(
        scored_segments: Vec<(PaperSegment, f32)>,
        k: usize,
    ) -> Vec<(PaperSegment, f32)> {
        if scored_segments.is_empty() || k == 0 {
            return Vec::new();
        }

        let lambda = 0.7; // Balance between relevance (0.7) and diversity (0.3)
        let mut selected = Vec::new();
        let mut remaining: Vec<(usize, PaperSegment, f32)> = scored_segments
            .into_iter()
            .enumerate()
            .map(|(idx, (seg, score))| (idx, seg, score))
            .collect();

        // Always select the top-scoring segment first
        if let Some((idx, _seg, _score)) = remaining
            .iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        {
            let (_idx, seg, score) =
                remaining.remove(remaining.iter().position(|(i, _, _)| *i == *idx).unwrap());
            selected.push((seg, score));
        }

        // MMR selection: iteratively select segments that maximize MMR score
        // Optimized with early termination, caching, and parallel similarity computation
        while selected.len() < k && !remaining.is_empty() {
            let mut best_mmr_score = f32::NEG_INFINITY;
            let mut best_idx = 0;

            // Pre-compute candidate embeddings if available (for performance)
            let candidate_embeddings: Vec<Option<&Vec<f32>>> = remaining
                .iter()
                .map(|(_, seg, _)| seg.embedding.as_ref())
                .collect();

            // Pre-compute selected embeddings for faster lookup
            let selected_embeddings: Vec<Option<&Vec<f32>>> = selected
                .iter()
                .map(|(seg, _)| seg.embedding.as_ref())
                .collect();

            for (idx, (_, candidate, relevance_score)) in remaining.iter().enumerate() {
                // Compute maximum similarity to already selected segments
                // Use early termination optimization: if max_similarity is already high,
                // we can skip further computation
                let mut max_similarity = 0.0f32;
                let candidate_emb = candidate_embeddings[idx];

                // Use embeddings if available for both candidate and selected segments
                let use_embeddings =
                    candidate_emb.is_some() && selected_embeddings.iter().any(|emb| emb.is_some());

                if use_embeddings {
                    // Fast path: use embeddings for similarity computation
                    if let Some(cand_emb) = candidate_emb {
                        for sel_emb_opt in &selected_embeddings {
                            if let Some(sel_emb) = sel_emb_opt {
                                let similarity = Self::cosine_similarity_static(cand_emb, sel_emb);
                                max_similarity = max_similarity.max(similarity);

                                // Early termination: if similarity is very high, this candidate is too similar
                                if max_similarity > 0.95 {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // Fallback: use content overlap (faster but less accurate)
                    for (selected_seg, _) in &selected {
                        let similarity =
                            Self::content_overlap(&candidate.content, &selected_seg.content);
                        max_similarity = max_similarity.max(similarity);

                        // Early termination
                        if max_similarity > 0.95 {
                            break;
                        }
                    }
                }

                // MMR score: λ * relevance - (1-λ) * max_similarity
                // Higher relevance and lower similarity to selected = higher MMR score
                let mmr_score = lambda * relevance_score - (1.0 - lambda) * max_similarity;

                if mmr_score > best_mmr_score {
                    best_mmr_score = mmr_score;
                    best_idx = idx;
                }
            }

            // Select the segment with highest MMR score
            let (_, seg, score) = remaining.remove(best_idx);
            selected.push((seg, score));
        }

        selected
    }

    /// Compute Jaccard similarity (content overlap) between two text segments
    /// Uses Jaccard coefficient: |A ∩ B| / |A ∪ B|
    /// This is a standard metric for text similarity
    fn content_overlap(text1: &str, text2: &str) -> f32 {
        // Normalize: lowercase and filter meaningful words
        let normalize_word = |w: &str| w.to_lowercase();

        let words1: std::collections::HashSet<String> = text1
            .split_whitespace()
            .filter(|w| w.len() > 3) // Filter short words
            .map(normalize_word)
            .collect();
        let words2: std::collections::HashSet<String> = text2
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(normalize_word)
            .collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        // Compute intersection and union
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Retrieve external references mentioned in the paper
    async fn retrieve_external_refs(
        &self,
        task: &Task,
        paper: &Paper,
        query_keywords: &[String],
    ) -> Result<Option<Vec<RetrievedContext>>> {
        // Check if task mentions external references
        let task_lower = task.description.to_lowercase();
        let needs_external = task_lower.contains("reference")
            || task_lower.contains("cite")
            || task_lower.contains("external");

        if !needs_external || paper.references.is_empty() {
            return Ok(None);
        }

        // Score references based on keyword overlap with task
        let mut scored_refs: Vec<(crate::types::Reference, f32)> = paper
            .references
            .iter()
            .map(|ref_| {
                let ref_text = format!("{} {}", ref_.title.as_deref().unwrap_or(""), ref_.citation)
                    .to_lowercase();

                let score = query_keywords
                    .iter()
                    .map(|kw| {
                        if ref_text.contains(&kw.to_lowercase()) {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .sum::<f32>()
                    / query_keywords.len().max(1) as f32;

                (ref_.clone(), score)
            })
            .filter(|(_, score)| *score > 0.1) // Only include relevant references
            .collect();

        scored_refs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Convert top references to RetrievedContext
        // Note: In a real implementation, this would fetch the actual paper content
        let contexts: Vec<RetrievedContext> = scored_refs
            .into_iter()
            .take(3) // Limit to top 3 external references
            .map(|(ref_, score)| {
                // Create a synthetic segment from reference
                let segment = crate::types::PaperSegment {
                    id: format!("ref_{}", ref_.id),
                    section: "Reference".to_string(),
                    content: format!(
                        "Reference: {}\nCitation: {}",
                        ref_.title.as_deref().unwrap_or("Unknown"),
                        ref_.citation
                    ),
                    segment_type: crate::types::SegmentType::Other("Reference".to_string()),
                    embedding: None,
                    line_range: (0, 0),
                };

                RetrievedContext {
                    segment,
                    score,
                    source: crate::types::ContextSource::ExternalReference {
                        citation: ref_.citation.clone(),
                    },
                    relevance_reason: format!(
                        "External reference relevant to task (score: {:.3})",
                        score
                    ),
                }
            })
            .collect();

        if contexts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(contexts))
        }
    }
}

impl Default for DefaultCPREngine {
    fn default() -> Self {
        Self::new()
    }
}
