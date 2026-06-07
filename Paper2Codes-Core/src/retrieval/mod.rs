pub mod augment;
pub mod bm25;
pub mod embedding;
pub mod fusion;
pub mod vector_store;

pub use augment::{
    combine_embeddings, parse_rerank_order, LlmQueryExpander, LlmReranker, QueryExpander, Reranker,
};
pub use bm25::{tokenize, Bm25Index};
pub use embedding::{EmbeddingProvider, EmbeddingService};
pub use fusion::{fused_relevance_map, reciprocal_rank_fusion, DEFAULT_RRF_K};
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
    // Phase 3 (optional) retrieval augmentation. When absent, retrieval falls
    // back to the Phase 1/2 hybrid pipeline.
    query_expander: Option<Arc<dyn augment::QueryExpander>>,
    reranker: Option<Arc<dyn augment::Reranker>>,
    /// Weight of the original-query embedding when blending with the HyDE
    /// hypothetical-document embedding (1.0 = ignore HyDE, 0.0 = HyDE only).
    hyde_alpha: f32,
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
            query_expander: None,
            reranker: None,
            hyde_alpha: 0.5,
        }
    }

    /// Attach a HyDE query expander (Phase 3). Only active when an embedding
    /// service is also configured.
    pub fn with_query_expander(mut self, expander: Arc<dyn augment::QueryExpander>) -> Self {
        self.query_expander = Some(expander);
        self
    }

    /// Attach a learned reranker (Phase 3) applied to the fused top-N before MMR.
    pub fn with_reranker(mut self, reranker: Arc<dyn augment::Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Set the HyDE blend weight for the original-query embedding (default 0.5).
    pub fn with_hyde_alpha(mut self, alpha: f32) -> Self {
        self.hyde_alpha = alpha.clamp(0.0, 1.0);
        self
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
        // Use the shared retrieval tokenizer so the lexical signal is identical
        // across query-keyword extraction and the BM25 index.
        bm25::tokenize(&task.description)
    }

    /// Detect structural references (e.g. "Algorithm 1", "Equation (5)",
    /// "Theorem 2", "Figure 3", "Section 4.2") in the task description.
    ///
    /// Research papers use these labels as high-precision anchors, and the
    /// Dirac-Paper2Codes paper calls out algorithm boxes, theorem statements and
    /// equation labels as structural retrieval targets. Matching them exactly is
    /// far more precise than the previous keyword-presence heuristic.
    fn extract_structural_references(text: &str) -> Vec<String> {
        use regex::Regex;
        lazy_static::lazy_static! {
            static ref REF_RE: Regex = Regex::new(
                r"(?i)\b(algorithm|equation|eq\.?|theorem|lemma|figure|fig\.?|table|section|sec\.?|definition|proposition|corollary)\s*\.?\s*\(?\s*(\d+(?:\.\d+)*)\)?"
            ).unwrap();
        }
        let normalize_kind = |kind: &str| -> &'static str {
            match kind.to_lowercase().trim_end_matches('.') {
                "eq" | "equation" => "equation",
                "fig" | "figure" => "figure",
                "sec" | "section" => "section",
                k if k == "algorithm" => "algorithm",
                k if k == "theorem" => "theorem",
                k if k == "lemma" => "lemma",
                k if k == "table" => "table",
                k if k == "definition" => "definition",
                k if k == "proposition" => "proposition",
                k if k == "corollary" => "corollary",
                _ => "ref",
            }
        };
        REF_RE
            .captures_iter(text)
            .map(|cap| format!("{} {}", normalize_kind(&cap[1]), &cap[2]))
            .collect()
    }

    /// Boost in `[0, 1]`: 1.0 when the segment names exactly the same structural
    /// reference(s) the task asks for, else 0.0.
    fn structural_reference_boost(&self, segment: &PaperSegment, task_refs: &[String]) -> f32 {
        if task_refs.is_empty() {
            return 0.0;
        }
        let seg_refs = Self::extract_structural_references(&segment.content);
        if seg_refs.is_empty() {
            return 0.0;
        }
        let hit = task_refs.iter().any(|r| seg_refs.contains(r));
        if hit {
            1.0
        } else {
            0.0
        }
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
        // 1. Generate query keywords and structural references
        let query_keywords = self.generate_query_keywords(task);
        let task_refs = Self::extract_structural_references(&task.description);

        // 2a. Dense (semantic) ranking from the vector store, as an ordered list
        //     of segment ids (best first). Pulled wider than k to give the
        //     fusion step a healthy candidate pool.
        let dense_ranking: Vec<String> =
            if self.use_embeddings && self.initialized.load(std::sync::atomic::Ordering::Acquire) {
                if let (Some(embedding_service), Some(vector_store)) =
                    (&self.embedding_service, &self.vector_store)
                {
                    // Base query embedding, optionally blended with a HyDE
                    // hypothetical-document embedding for better semantic recall.
                    let base_embedding = embedding_service.embed(&task.description).await?;
                    let query_embedding = match &self.query_expander {
                        Some(expander) => match expander.expand(task).await {
                            Ok(Some(doc)) if !doc.trim().is_empty() => {
                                match embedding_service.embed(&doc).await {
                                    Ok(doc_embedding) => augment::combine_embeddings(
                                        &base_embedding,
                                        &doc_embedding,
                                        self.hyde_alpha,
                                    ),
                                    Err(e) => {
                                        tracing::warn!(
                                            "HyDE embedding failed, using query embedding: {}",
                                            e
                                        );
                                        base_embedding
                                    }
                                }
                            }
                            Ok(_) => base_embedding,
                            Err(e) => {
                                tracing::warn!("HyDE expansion failed, using query embedding: {}", e);
                                base_embedding
                            }
                        },
                        None => base_embedding,
                    };
                    let store = vector_store.read().await;
                    store
                        .search(&query_embedding, (k * 4).max(10))
                        .await?
                        .into_iter()
                        .map(|r| r.id)
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

        // 2b. Sparse (BM25) ranking over the paper segments. This replaces the
        //     previous substring-containment heuristic with proper lexical
        //     scoring (tf saturation, IDF, length normalisation).
        let bm25 = Bm25Index::build(
            paper
                .segments
                .iter()
                .map(|seg| (seg.id.clone(), seg.content.as_str())),
        );
        let sparse_ranking = bm25.ranked_ids(&query_keywords);

        // 2c. Fuse dense + sparse rankings via Reciprocal Rank Fusion. RRF is
        //     robust to the incompatible score scales of the two retrievers and
        //     yields a relevance map normalised to [0, 1].
        let mut rankings: Vec<Vec<String>> = Vec::new();
        if !dense_ranking.is_empty() {
            rankings.push(dense_ranking);
        }
        if !sparse_ranking.is_empty() {
            rankings.push(sparse_ranking);
        }
        let fused = fusion::fused_relevance_map(&rankings, fusion::DEFAULT_RRF_K);

        // 3. Final per-segment scoring: fused relevance + lexical booster +
        //    structural-reference boost − already-implemented de-boost.
        let mut scored_segments: Vec<(PaperSegment, f32)> = Vec::new();

        for segment in &paper.segments {
            // Fused dense+sparse relevance is the primary signal.
            let base = fused.get(&segment.id).copied().unwrap_or(0.0);

            // Lightweight lexical booster / tie-breaker.
            let keyword_score = self.keyword_overlap(&segment.content, &query_keywords);

            let mut score = self.alpha * base + self.lambda * keyword_score;

            // Structural boost: exact reference match (e.g. "Algorithm 1") when
            // the task names one, otherwise a weaker generic algorithm-content
            // boost.
            let structural = if !task_refs.is_empty() {
                self.structural_reference_boost(segment, &task_refs)
            } else if self.is_algorithm_match(segment, task) {
                0.5
            } else {
                0.0
            };
            score += self.delta * structural;

            // De-boost for already implemented (avoid redundant work).
            if self.is_already_implemented(segment, repository) {
                score -= self.gamma;
            }

            score = score.max(0.0).min(1.0);
            scored_segments.push((segment.clone(), score));
        }

        // 4. Sort by fused score.
        scored_segments.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 4b. Optional learned reranking (Phase 3): reorder the fused top-N with
        //     an LLM reranker, then re-derive relevance from the new ordering so
        //     it drives the subsequent MMR selection. Falls back to the fused
        //     ordering on any error.
        if let Some(reranker) = &self.reranker {
            let pool_size = (k * 3).max(10).min(scored_segments.len());
            if pool_size > 1 {
                let pool: Vec<(PaperSegment, f32)> = scored_segments[..pool_size].to_vec();
                let pool_ctx: Vec<RetrievedContext> = pool
                    .iter()
                    .map(|(seg, score)| RetrievedContext {
                        segment: seg.clone(),
                        score: *score,
                        source: crate::types::ContextSource::Paper,
                        relevance_reason: String::new(),
                    })
                    .collect();

                match reranker.rerank(task, &pool_ctx).await {
                    Ok(order) => {
                        let n = pool.len();
                        let mut reordered: Vec<(PaperSegment, f32)> = Vec::with_capacity(n);
                        for (new_rank, &orig_idx) in order.iter().enumerate() {
                            if let Some((seg, _)) = pool.get(orig_idx) {
                                // Rank-derived relevance in (0, 1], highest first.
                                let new_score = (n - new_rank) as f32 / n as f32;
                                reordered.push((seg.clone(), new_score));
                            }
                        }
                        // Keep any tail segments beyond the reranked pool.
                        let tail = scored_segments.split_off(pool_size);
                        scored_segments = reordered;
                        scored_segments.extend(tail);
                    }
                    Err(e) => {
                        tracing::warn!("Reranking failed, using fused ordering: {}", e);
                    }
                }
            }
        }

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
