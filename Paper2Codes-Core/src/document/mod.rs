pub mod classifier;
pub mod extractor;
pub mod parser;
pub mod segmenter;

use crate::domain::detector::DomainDetector;
use crate::error::Result;
use crate::github::client::GitHubClient;
use crate::github::extractor::extract_github_urls;
use crate::retrieval::embedding::EmbeddingService;
use crate::storage::manager::StorageManager;
use crate::types::Paper;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

pub use classifier::DomainClassifier;
pub use extractor::ContentExtractor;
pub use parser::DocumentParser;
pub use segmenter::PaperSegmenter;

/// Main document processor that coordinates parsing, segmentation, extraction, and storage
pub struct DocumentProcessor {
    parser: DocumentParser,
    segmenter: PaperSegmenter,
    extractor: ContentExtractor,
    classifier: DomainClassifier,
    domain_detector: Option<Arc<DomainDetector>>,
    embedding_service: Option<Arc<EmbeddingService>>,
    storage_manager: Option<Arc<StorageManager>>,
    github_client: Option<Arc<GitHubClient>>,
}

impl DocumentProcessor {
    pub fn new() -> Self {
        Self {
            parser: DocumentParser::new(),
            segmenter: PaperSegmenter::new(),
            extractor: ContentExtractor::new(),
            classifier: DomainClassifier::new(),
            domain_detector: None,
            embedding_service: None,
            storage_manager: None,
            github_client: None,
        }
    }

    /// Create a new document processor with optional services
    pub fn with_services(
        domain_detector: Option<Arc<DomainDetector>>,
        embedding_service: Option<Arc<EmbeddingService>>,
        storage_manager: Option<Arc<StorageManager>>,
        github_client: Option<Arc<GitHubClient>>,
    ) -> Self {
        Self {
            parser: DocumentParser::new(),
            segmenter: PaperSegmenter::new(),
            extractor: ContentExtractor::new(),
            classifier: DomainClassifier::new(),
            domain_detector,
            embedding_service,
            storage_manager,
            github_client,
        }
    }

    /// Parse a PDF file and return a fully processed Paper with storage integration
    pub async fn parse_pdf(&self, path: &Path) -> Result<Paper> {
        let mut paper = self.parser.parse_pdf(path).await?;
        self.process_paper_full(&mut paper).await?;
        Ok(paper)
    }

    /// Parse text content and return a fully processed Paper with storage integration
    pub async fn parse_text(&self, content: &str, title: Option<String>) -> Result<Paper> {
        let mut paper = self.parser.parse_text(content, title).await?;
        self.process_paper_full(&mut paper).await?;
        Ok(paper)
    }

    /// Full processing pipeline: segment, extract, classify, generate embeddings, and store
    pub async fn process_paper_full(&self, paper: &mut Paper) -> Result<()> {
        info!(
            "Starting full paper processing pipeline for: {}",
            paper.title
        );

        // Step 1: Segment the paper into sections
        info!("Segmenting paper into sections...");
        self.segmenter.segment(paper)?;
        info!("Created {} segments", paper.segments.len());

        // Step 2: Extract algorithms and equations
        info!("Extracting algorithms and equations...");
        paper.algorithms = self.extractor.extract_algorithms(paper);
        paper.equations = self.extractor.extract_equations(paper);
        info!(
            "Extracted {} algorithms and {} equations",
            paper.algorithms.len(),
            paper.equations.len()
        );

        // Step 3: Extract GitHub URLs and fetch repository information
        if let Some(ref github_client) = self.github_client {
            info!("Extracting GitHub repository information...");
            self.extract_github_repositories(paper, github_client.as_ref())
                .await?;
        }

        // Step 4: Domain classification (use advanced detector if available)
        info!("Classifying domain...");
        if let Some(ref detector) = self.domain_detector {
            let text_chunks: Vec<String> = paper
                .segments
                .iter()
                .take(5) // Use first 5 segments for domain detection
                .map(|s| s.content.clone())
                .collect();

            match detector.detect_domain(&text_chunks).await {
                Ok(domain) => {
                    info!("Detected domain: {:?}", domain);
                    // Store domain information in paper metadata if needed
                    // This could be added to PaperMetadata structure
                }
                Err(e) => {
                    warn!("Domain detection failed: {}, using basic classifier", e);
                    // Fallback to basic classifier
                    let _ = self.classifier.classify(paper).await;
                }
            }
        } else {
            // Use basic classifier
            let _ = self.classifier.classify(paper).await;
        }

        // Step 5: Generate embeddings for segments
        if let Some(ref embedding_service) = self.embedding_service {
            info!(
                "Generating embeddings for {} segments...",
                paper.segments.len()
            );
            self.generate_embeddings(paper, embedding_service.as_ref())
                .await?;
        }

        // Step 6: Save to storage (SurrealDB)
        if let Some(ref storage) = self.storage_manager {
            if storage.is_connected().await {
                info!("Saving paper and segments to SurrealDB...");
                self.save_to_storage(paper, storage.as_ref()).await?;
                info!("Successfully saved paper to storage");
            } else {
                warn!("Storage manager is not connected, skipping storage save");
            }
        }

        info!("Paper processing pipeline completed successfully");
        Ok(())
    }

    /// Process an existing paper (segment and extract content) - basic version
    pub async fn process_paper(&self, paper: &mut Paper) -> Result<()> {
        // Segment the paper into sections
        self.segmenter.segment(paper)?;

        // Extract algorithms and equations
        paper.algorithms = self.extractor.extract_algorithms(paper);
        paper.equations = self.extractor.extract_equations(paper);

        Ok(())
    }

    /// Extract GitHub repositories from paper and fetch their information
    async fn extract_github_repositories(
        &self,
        paper: &mut Paper,
        github_client: &GitHubClient,
    ) -> Result<()> {
        // Combine all text to search for GitHub URLs
        let mut full_text = format!("{} {}", paper.title, paper.abstract_text);
        for segment in &paper.segments {
            full_text = format!("{} {}", full_text, segment.content);
        }

        let urls = extract_github_urls(&full_text);
        if urls.is_empty() {
            info!("No GitHub URLs found in paper");
            return Ok(());
        }

        info!("Found {} GitHub URLs in paper", urls.len());

        // Extract repository information and optionally fetch full details
        // GitHubClient is now thread-safe and can be used concurrently
        for url in &urls {
            if let Some((owner, repo)) = github_client.extract_repo_info_from_url(url) {
                info!("Found GitHub repository: {}/{}", owner, repo);

                // Optionally fetch full repository information
                // This is now safe to do since GitHubClient is thread-safe
                match github_client.get_repository_info(&owner, &repo).await {
                    Ok(repo_info) => {
                        info!(
                            "Fetched repository info for {}/{}: {}",
                            owner, repo, repo_info.full_name
                        );
                        // Store repository information in paper metadata if needed
                        // This information can be used later by agents for code generation context
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch repository info for {}/{}: {}",
                            owner,
                            repo,
                            e
                        );
                        // Continue processing other repositories even if one fails
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate embeddings for all segments
    async fn generate_embeddings(
        &self,
        paper: &mut Paper,
        embedding_service: &EmbeddingService,
    ) -> Result<()> {
        let texts: Vec<String> = paper.segments.iter().map(|s| s.content.clone()).collect();

        let embeddings = embedding_service.embed_batch(&texts).await?;

        // Assign embeddings to segments
        for (segment, embedding) in paper.segments.iter_mut().zip(embeddings.iter()) {
            segment.embedding = Some(embedding.clone());
        }

        info!("Generated embeddings for {} segments", paper.segments.len());
        Ok(())
    }

    /// Save paper and segments to SurrealDB storage
    async fn save_to_storage(&self, paper: &Paper, storage: &StorageManager) -> Result<()> {
        // Save paper
        storage.save_paper(paper).await?;

        // Save segments with embeddings
        for segment in &paper.segments {
            storage.save_segment(segment).await?;

            // Save embedding if available
            if let Some(ref embedding) = segment.embedding {
                storage
                    .save_embedding(&segment.id, embedding.clone())
                    .await?;
            }
        }

        info!(
            "Saved paper and {} segments to storage",
            paper.segments.len()
        );
        Ok(())
    }

    /// Segment a paper into sections
    pub fn segment_paper(&self, paper: &mut Paper) -> Result<()> {
        self.segmenter.segment(paper)
    }

    /// Extract algorithms from a paper
    pub fn extract_algorithms(&self, paper: &Paper) -> Vec<crate::types::Algorithm> {
        self.extractor.extract_algorithms(paper)
    }

    /// Extract equations from a paper
    pub fn extract_equations(&self, paper: &Paper) -> Vec<crate::types::Equation> {
        self.extractor.extract_equations(paper)
    }

    /// Classify the domain of a paper
    pub async fn classify_domain(&self, paper: &Paper) -> Result<crate::types::DomainContext> {
        self.classifier.classify(paper).await
    }
}

impl Default for DocumentProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_text_basic() {
        let processor = DocumentProcessor::new();
        let content = "Test Title\n\nAbstract\n\nThis is a test paper.";
        let result = processor
            .parse_text(content, Some("Test".to_string()))
            .await;
        assert!(result.is_ok());

        let paper = result.unwrap();
        assert_eq!(paper.title, "Test");
        assert!(!paper.segments.is_empty());
    }

    #[tokio::test]
    async fn test_parse_text_with_sections() {
        let processor = DocumentProcessor::new();
        let content = r#"Test Paper

Abstract

This is the abstract section.

Introduction

This is the introduction section with some content.

Methodology

We propose a new algorithm:

Algorithm 1: Test Algorithm
Input: x
Output: y
1: Initialize y = 0
2: For each element in x
3:   Add element to y
4: Return y

Results

The results show significant improvement.
"#;
        let result = processor.parse_text(content, None).await;
        assert!(result.is_ok());

        let paper = result.unwrap();
        assert!(!paper.title.is_empty());
        assert!(!paper.abstract_text.is_empty());
        assert!(paper.segments.len() > 1);

        // Check if algorithm was extracted
        assert!(
            !paper.algorithms.is_empty(),
            "Algorithm should be extracted"
        );
        assert!(
            paper.algorithms[0].name.contains("Algorithm"),
            "Algorithm name should be extracted"
        );
    }

    #[tokio::test]
    async fn test_algorithm_extraction() {
        let processor = DocumentProcessor::new();
        let content = r#"Algorithm 1: Sorting Algorithm
Input: Array A
Output: Sorted array
1: For i = 1 to n
2:   For j = i+1 to n
3:     If A[i] > A[j]
4:       Swap A[i] and A[j]
5: Return A
End Algorithm"#;

        let paper = processor
            .parse_text(content, Some("Test".to_string()))
            .await
            .unwrap();
        let algorithms = processor.extract_algorithms(&paper);

        assert!(
            !algorithms.is_empty(),
            "Should extract at least one algorithm"
        );
        assert!(
            algorithms[0].pseudocode.contains("For i"),
            "Should contain algorithm pseudocode"
        );
    }

    #[tokio::test]
    async fn test_equation_extraction() {
        let processor = DocumentProcessor::new();
        let content = "We define the function as f(x) = x^2 + 2x + 1\n\nThe loss is calculated as: L = ∑(y - ŷ)^2";

        let paper = processor
            .parse_text(content, Some("Test".to_string()))
            .await
            .unwrap();
        let equations = processor.extract_equations(&paper);

        assert!(
            !equations.is_empty(),
            "Equation extraction should yield at least one candidate"
        );
    }

    #[test]
    fn test_processor_creation() {
        let _processor = DocumentProcessor::new();
        let _processor2 = DocumentProcessor::default();
    }
}
