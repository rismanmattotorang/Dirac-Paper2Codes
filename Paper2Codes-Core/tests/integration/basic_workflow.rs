/// Integration tests for basic Paper2Codes workflow
use paper2codes::config::Config;
use paper2codes::coordinator::Coordinator;
use paper2codes::document::DocumentProcessor;
use paper2codes::types::Paper;

/// Test basic paper parsing
#[tokio::test]
async fn test_parse_text_paper() {
    let processor = DocumentProcessor::new();
    let content = "Abstract\n\nThis is a test paper about algorithms.\n\nIntroduction\n\nWe present a novel approach.";
    
    let result = processor.parse_text(content, Some("Test Paper".to_string())).await;
    assert!(result.is_ok());
    
    let paper = result.unwrap();
    assert_eq!(paper.title, "Test Paper");
    assert!(!paper.segments.is_empty());
}

/// Test paper segmentation
#[tokio::test]
async fn test_paper_segmentation() {
    let processor = DocumentProcessor::new();
    let content = "Abstract\n\nThis is an abstract.\n\nIntroduction\n\nThis is an introduction.";
    
    let mut paper = processor.parse_text(content, Some("Test".to_string())).await.unwrap();
    let result = processor.segment_paper(&mut paper);
    
    assert!(result.is_ok());
    assert!(!paper.segments.is_empty());
}

/// Test configuration loading
#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.llm.default_provider, "openrouter");
    assert!(config.agents.max_iterations > 0);
    assert!(config.agents.parallel_tasks > 0);
}

/// Test coordinator initialization
#[tokio::test]
async fn test_coordinator_initialization() {
    let config = Config::default();
    let result = Coordinator::new(config).await;
    
    // May fail without API keys, but should not panic
    if result.is_err() {
        println!("Coordinator initialization failed (expected without API keys)");
    }
}

/// Test task queue operations
#[test]
fn test_task_queue() {
    use paper2codes::coordinator::TaskQueue;
    use paper2codes::types::{Task, TaskType};
    
    let mut queue = TaskQueue::new();
    
    let task = Task::new(
        TaskType::Planning,
        "Test task".to_string(),
    );
    
    queue.add_task(task.clone());
    assert!(!queue.is_empty());
    assert_eq!(queue.total_count(), 1);
    
    let ready = queue.get_ready_tasks();
    assert_eq!(ready.len(), 1);
    
    queue.mark_completed(&task.id);
    assert!(queue.is_empty());
    assert_eq!(queue.get_completed().len(), 1);
}

/// Test retrieval system components
#[tokio::test]
async fn test_cpr_engine() {
    use paper2codes::retrieval::DefaultCPREngine;
    use paper2codes::types::{Task, TaskType, Paper, Repository, PaperSegment, SegmentType};
    use std::path::PathBuf;
    
    let mut engine = DefaultCPREngine::new();
    
    let paper = Paper {
        id: "test".to_string(),
        title: "Test".to_string(),
        abstract_text: "Test abstract".to_string(),
        segments: vec![
            PaperSegment {
                id: "seg1".to_string(),
                section: "Introduction".to_string(),
                content: "This paper introduces a novel algorithm for sorting".to_string(),
                segment_type: SegmentType::Introduction,
                embedding: None,
                line_range: (0, 10),
            }
        ],
        algorithms: vec![],
        equations: vec![],
        figures: vec![],
        tables: vec![],
        references: vec![],
        metadata: paper2codes::types::PaperMetadata {
            authors: vec![],
            year: None,
            venue: None,
            keywords: vec![],
            file_path: None,
        },
    };
    
    let task = Task::new(
        TaskType::Coding { module_id: "test".to_string() },
        "Implement sorting algorithm".to_string(),
    );
    
    let repository = Repository::new(PathBuf::from("./output"));
    
    // Initialize without embeddings (basic keyword-based retrieval)
    engine.initialize(&paper).await.ok();
    
    let result = engine.retrieve(&task, &paper, &repository, 5).await;
    assert!(result.is_ok());
    
    let contexts = result.unwrap();
    assert!(!contexts.is_empty());
}

/// Test error handling
#[test]
fn test_error_types() {
    use paper2codes::error::{Paper2CodesError, DocumentError};
    
    let error = Paper2CodesError::Document(
        DocumentError::ParseFailed("Test error".to_string())
    );
    
    assert!(error.to_string().contains("Test error"));
}

/// Test storage manager (without actual connection)
#[test]
fn test_storage_manager_creation() {
    use paper2codes::storage::StorageManager;
    
    let manager = StorageManager::new();
    assert!(manager.is_ok());
}

/// Test vector store operations
#[tokio::test]
async fn test_vector_store() {
    use paper2codes::retrieval::VectorStoreBuilder;
    
    let mut store = VectorStoreBuilder::new(128).build();
    
    // Add a vector
    let id = "vec1".to_string();
    let vector = vec![0.1; 128];
    let metadata = paper2codes::retrieval::vector_store::SegmentMetadata {
        paper_id: "paper1".to_string(),
        section: "Introduction".to_string(),
        segment_type: "Introduction".to_string(),
    };
    
    let mut store_write = store.write().await;
    let result = store_write.add(id.clone(), vector.clone(), metadata).await;
    assert!(result.is_ok());
    
    // Search
    let search_result = store_write.search(&vector, 1).await;
    assert!(search_result.is_ok());
    
    let results = search_result.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
}

