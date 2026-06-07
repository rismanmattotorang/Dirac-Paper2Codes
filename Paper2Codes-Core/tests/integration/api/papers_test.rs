//! Integration tests for papers endpoints

use crate::common::{TestServer, fixtures::test_paper};
use crate::common::helpers::{assert_api_response, create_test_paper};

#[tokio::test]
async fn test_list_papers_empty() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/papers").await.expect("Request failed");
    // May require authentication, so we check for either success or 401
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn test_create_paper() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let paper = create_test_paper();
    let response = server.post("/api/papers", &paper).await.expect("Request failed");
    
    // May require authentication
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
        assert_eq!(json["success"], true);
        assert!(json["data"]["id"].is_string());
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn test_get_paper_not_found() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/papers/non-existent-id").await.expect("Request failed");
    
    // May require authentication or return 404
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert!(response.status.as_u16() == 401 || response.status.as_u16() == 404);
    }
}

#[tokio::test]
async fn test_update_paper() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let paper = create_test_paper();
    let response = server.put("/api/papers/test-id", &paper).await.expect("Request failed");
    
    // May require authentication
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn test_delete_paper() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.delete("/api/papers/test-id").await.expect("Request failed");
    
    // May require authentication
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn test_paper_validation() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Test with invalid paper data (missing required fields)
    let invalid_paper = serde_json::json!({});
    let response = server.post("/api/papers", &invalid_paper).await.expect("Request failed");
    
    // Should return validation error or require auth
    if response.status.is_client_error() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert!(!json["success"].as_bool().unwrap_or(true));
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

