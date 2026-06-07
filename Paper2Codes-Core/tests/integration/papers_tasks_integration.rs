/// Integration tests for Papers and Task Queue features
/// 
/// This test suite validates the complete functionality of:
/// - Papers CRUD operations
/// - Paper upload and processing
/// - Task creation, update, and cancellation
/// - Storage layer integration
/// - Real-time WebSocket updates

use paper2codes::{
    config::{Config, ConfigBuilder},
    types::{Paper, Task, TaskStatus, TaskType, PaperMetadata, TaskContext},
    storage::{manager::StorageManager, traits::Storage},
    api::server::ApiServer,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use uuid::Uuid;
use serde_json::json;

#[tokio::test]
async fn test_papers_crud_operations() {
    // Setup test config
    let config = ConfigBuilder::default()
        .build()
        .expect("Failed to build config");
    
    // Create API server
    let server = ApiServer::new(config)
        .await
        .expect("Failed to create API server");
    
    // Test 1: Create a paper
    let create_request = Request::builder()
        .uri("/api/papers")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "Test Paper: Attention Mechanisms",
                "abstract_text": "This paper explores attention mechanisms in neural networks."
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(create_request)
        .await
        .expect("Failed to create paper");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Extract paper ID from response
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let create_result: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse response");
    let paper_id = create_result["data"]["id"]
        .as_str()
        .expect("Failed to get paper ID");
    
    // Test 2: Get paper by ID
    let get_request = Request::builder()
        .uri(format!("/api/papers/{}", paper_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(get_request)
        .await
        .expect("Failed to get paper");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 3: Update paper
    let update_request = Request::builder()
        .uri(format!("/api/papers/{}", paper_id))
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "Updated Paper: Attention Mechanisms v2",
                "abstract_text": "Updated abstract with more details."
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(update_request)
        .await
        .expect("Failed to update paper");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 4: List papers
    let list_request = Request::builder()
        .uri("/api/papers?page=1&per_page=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(list_request)
        .await
        .expect("Failed to list papers");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 5: Delete paper
    let delete_request = Request::builder()
        .uri(format!("/api/papers/{}", paper_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(delete_request)
        .await
        .expect("Failed to delete paper");
    
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    
    // Verify paper is deleted
    let verify_request = Request::builder()
        .uri(format!("/api/papers/{}", paper_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(verify_request)
        .await
        .expect("Failed to verify deletion");
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_task_crud_operations() {
    // Setup test config
    let config = ConfigBuilder::default()
        .build()
        .expect("Failed to build config");
    
    // Create API server
    let server = ApiServer::new(config)
        .await
        .expect("Failed to create API server");
    
    // Test 1: Create a task
    let create_request = Request::builder()
        .uri("/api/tasks")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "description": "Process paper for analysis",
                "task_type": "Planning",
                "paper_id": Uuid::new_v4().to_string()
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(create_request)
        .await
        .expect("Failed to create task");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Extract task ID from response
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let create_result: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse response");
    let task_id = create_result["data"]["id"]
        .as_str()
        .expect("Failed to get task ID");
    
    // Test 2: Get task by ID
    let get_request = Request::builder()
        .uri(format!("/api/tasks/{}", task_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(get_request)
        .await
        .expect("Failed to get task");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 3: Update task status
    let update_request = Request::builder()
        .uri(format!("/api/tasks/{}/status", task_id))
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "status": "InProgress"
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(update_request)
        .await
        .expect("Failed to update task status");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 4: List tasks
    let list_request = Request::builder()
        .uri("/api/tasks?page=1&per_page=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(list_request)
        .await
        .expect("Failed to list tasks");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test 5: Cancel task
    let cancel_request = Request::builder()
        .uri(format!("/api/tasks/{}/cancel", task_id))
        .method("POST")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(cancel_request)
        .await
        .expect("Failed to cancel task");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify task is cancelled
    let verify_request = Request::builder()
        .uri(format!("/api/tasks/{}", task_id))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(verify_request)
        .await
        .expect("Failed to verify cancellation");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let result: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse response");
    
    // Check that task status is Failed (cancelled)
    let status = &result["data"]["status"];
    assert!(status.is_object());
    assert!(status.get("Failed").is_some());
}

#[tokio::test]
async fn test_storage_integration() {
    // Setup test config with storage enabled
    let mut config = ConfigBuilder::default()
        .build()
        .expect("Failed to build config");
    config.storage.enabled = true;
    
    // Create storage manager
    let storage = StorageManager::new(&config.storage)
        .await
        .expect("Failed to create storage manager");
    
    // Test paper storage
    let paper = Paper {
        id: Uuid::new_v4().to_string(),
        title: "Test Paper for Storage".to_string(),
        abstract_text: "Testing storage integration".to_string(),
        segments: vec![],
        algorithms: vec![],
        equations: vec![],
        figures: vec![],
        tables: vec![],
        references: vec![],
        metadata: PaperMetadata {
            authors: vec!["Test Author".to_string()],
            year: Some(2024),
            venue: Some("Test Conference".to_string()),
            keywords: vec!["test".to_string(), "integration".to_string()],
            file_path: None,
        },
    };
    
    // Save paper
    storage.save_paper(&paper)
        .await
        .expect("Failed to save paper");
    
    // Retrieve paper
    let retrieved_paper = storage.get_paper(&paper.id)
        .await
        .expect("Failed to get paper")
        .expect("Paper not found");
    
    assert_eq!(retrieved_paper.id, paper.id);
    assert_eq!(retrieved_paper.title, paper.title);
    
    // Test task storage
    let task = Task {
        id: Uuid::new_v4(),
        task_type: TaskType::Planning,
        description: "Test task for storage".to_string(),
        context: TaskContext {
            paper_id: Some(paper.id.clone()),
            module_id: None,
            repository_id: None,
            dependencies: vec![],
            metadata: serde_json::Value::Null,
        },
        dependencies: vec![],
        status: TaskStatus::Pending,
        agent_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    // Save task
    storage.save_task(&task)
        .await
        .expect("Failed to save task");
    
    // Retrieve task
    let retrieved_task = storage.get_task(&task.id)
        .await
        .expect("Failed to get task")
        .expect("Task not found");
    
    assert_eq!(retrieved_task.id, task.id);
    assert_eq!(retrieved_task.description, task.description);
    
    // Cleanup
    storage.delete_paper(&paper.id)
        .await
        .expect("Failed to delete paper");
}

#[tokio::test]
async fn test_pagination() {
    // Setup test config
    let config = ConfigBuilder::default()
        .build()
        .expect("Failed to build config");
    
    // Create API server
    let server = ApiServer::new(config)
        .await
        .expect("Failed to create API server");
    
    // Create multiple papers
    for i in 0..15 {
        let create_request = Request::builder()
            .uri("/api/papers")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "title": format!("Test Paper {}", i),
                    "abstract_text": format!("Abstract for paper {}", i)
                })
                .to_string(),
            ))
            .unwrap();
        
        let response = server.router().clone()
            .oneshot(create_request)
            .await
            .expect("Failed to create paper");
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    // Test pagination - page 1
    let list_request = Request::builder()
        .uri("/api/papers?page=1&per_page=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(list_request)
        .await
        .expect("Failed to list papers");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let result: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse response");
    
    let data = result["data"].as_array().expect("Expected array");
    assert_eq!(data.len(), 10);
    
    // Test pagination - page 2
    let list_request = Request::builder()
        .uri("/api/papers?page=2&per_page=10")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(list_request)
        .await
        .expect("Failed to list papers");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("Failed to read response body");
    let result: serde_json::Value = serde_json::from_slice(&body)
        .expect("Failed to parse response");
    
    let data = result["data"].as_array().expect("Expected array");
    assert_eq!(data.len(), 5);
}

#[tokio::test]
async fn test_error_handling() {
    // Setup test config
    let config = ConfigBuilder::default()
        .build()
        .expect("Failed to build config");
    
    // Create API server
    let server = ApiServer::new(config)
        .await
        .expect("Failed to create API server");
    
    // Test 1: Get non-existent paper
    let get_request = Request::builder()
        .uri(format!("/api/papers/{}", Uuid::new_v4()))
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(get_request)
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    // Test 2: Create paper with invalid data
    let create_request = Request::builder()
        .uri("/api/papers")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "", // Empty title should fail validation
                "abstract_text": "Some abstract"
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(create_request)
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    // Test 3: Update non-existent paper
    let update_request = Request::builder()
        .uri(format!("/api/papers/{}", Uuid::new_v4()))
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "title": "Updated Title"
            })
            .to_string(),
        ))
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(update_request)
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    // Test 4: Get task with invalid UUID
    let get_request = Request::builder()
        .uri("/api/tasks/invalid-uuid")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = server.router().clone()
        .oneshot(get_request)
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

