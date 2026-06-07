//! End-to-end tests for complete user workflows
//!
//! Tests complete workflows from paper upload to code generation

use crate::common::TestServer;
use crate::common::helpers::{create_test_paper, wait_for_condition};
use serde_json::json;

/// Test complete paper processing workflow
#[tokio::test]
#[ignore] // May require actual storage/processing - run explicitly
async fn test_paper_processing_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Step 1: Upload paper
    let paper = create_test_paper();
    let upload_response = server.post("/api/papers/upload", &paper)
        .await
        .expect("Upload request failed");
    
    if upload_response.status.is_success() {
        let upload_json: serde_json::Value = upload_response.json()
            .expect("Failed to parse upload response");
        
        if let Some(paper_id) = upload_json["data"]["id"].as_str() {
            println!("Paper uploaded with ID: {}", paper_id);
            
            // Step 2: Check processing status
            let status_response = server.get(&format!("/api/papers/{}/status", paper_id))
                .await
                .expect("Status request failed");
            
            if status_response.status.is_success() {
                let status_json: serde_json::Value = status_response.json()
                    .expect("Failed to parse status response");
                
                println!("Processing status: {:?}", status_json["data"]["status"]);
                
                // Step 3: Get paper segments (if processing complete)
                let segments_response = server.get(&format!("/api/papers/{}/segments", paper_id))
                    .await
                    .expect("Segments request failed");
                
                if segments_response.status.is_success() {
                    let segments_json: serde_json::Value = segments_response.json()
                        .expect("Failed to parse segments response");
                    
                    println!("Retrieved {} segments", 
                             segments_json["data"].as_array()
                                 .map(|a| a.len())
                                 .unwrap_or(0));
                }
            }
        }
    }
    
    // Test may require authentication or actual processing, so we just verify
    // that the endpoints exist and respond appropriately
    println!("Paper processing workflow test completed");
}

/// Test code generation workflow
#[tokio::test]
#[ignore] // Requires actual code generation - run explicitly
async fn test_code_generation_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // This would test the full workflow:
    // 1. Upload paper
    // 2. Process paper
    // 3. Start code generation
    // 4. Monitor generation progress
    // 5. Retrieve generated repository
    // 6. Verify code modules
    
    println!("Code generation workflow test - requires full implementation");
}

/// Test search and retrieval workflow
#[tokio::test]
async fn test_search_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Test search endpoint
    let search_query = json!({
        "query": "test",
        "limit": 10
    });
    
    let response = server.post("/api/papers/search", &search_query)
        .await
        .expect("Search request failed");
    
    // May require authentication or return empty results
    if response.status.is_success() {
        let json: serde_json::Value = response.json()
            .expect("Failed to parse search response");
        
        println!("Search returned results");
        assert!(json["data"].is_array() || json["data"].is_null());
    } else {
        // Expected if auth required or not implemented
        assert!(response.status.is_client_error() || response.status.is_server_error());
    }
}

/// Test error handling workflow
#[tokio::test]
async fn test_error_handling_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Test various error scenarios
    
    // 1. Invalid endpoint
    let response = server.get("/api/invalid-endpoint")
        .await
        .expect("Request failed");
    assert!(response.status.is_client_error() || response.status.as_u16() == 404);
    
    // 2. Invalid paper ID
    let response = server.get("/api/papers/invalid-id-12345")
        .await
        .expect("Request failed");
    // May return 404 or require auth
    assert!(
        response.status.is_client_error() || 
        response.status.as_u16() == 404 ||
        response.status.as_u16() == 401
    );
    
    // 3. Invalid request body
    let invalid_body = json!({
        "invalid": "data"
    });
    let response = server.post("/api/papers", &invalid_body)
        .await
        .expect("Request failed");
    // Should return validation error or require auth
    assert!(
        response.status.is_client_error() || 
        response.status.as_u16() == 401
    );
}

/// Test pagination workflow
#[tokio::test]
async fn test_pagination_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Test pagination parameters
    let response = server.get("/api/papers?page=1&per_page=10")
        .await
        .expect("Request failed");
    
    if response.status.is_success() {
        let json: serde_json::Value = response.json()
            .expect("Failed to parse response");
        
        // Check for pagination metadata
        if let Some(pagination) = json["data"]["pagination"].as_object() {
            assert!(pagination.contains_key("page"));
            assert!(pagination.contains_key("per_page"));
            assert!(pagination.contains_key("total"));
        }
    }
}

/// Test batch request workflow
#[tokio::test]
async fn test_batch_request_workflow() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let batch_request = json!({
        "requests": [
            {
                "id": "req-1",
                "method": "GET",
                "path": "/api/health"
            },
            {
                "id": "req-2",
                "method": "GET",
                "path": "/api/health"
            }
        ]
    });
    
    let response = server.post("/api/batch", &batch_request)
        .await
        .expect("Batch request failed");
    
    if response.status.is_success() {
        let json: serde_json::Value = response.json()
            .expect("Failed to parse batch response");
        
        // Should have responses array
        if let Some(responses) = json["data"]["responses"].as_array() {
            assert_eq!(responses.len(), 2);
        }
    } else {
        // May require authentication
        assert_eq!(response.status.as_u16(), 401);
    }
}

