//! Integration tests for tasks endpoints

use crate::common::TestServer;
use crate::common::helpers::assert_api_response;

#[tokio::test]
async fn test_list_tasks() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/tasks").await.expect("Request failed");
    
    // May require authentication
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert_eq!(response.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn test_get_task() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/tasks/test-id").await.expect("Request failed");
    
    // May require authentication or return 404
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        assert_api_response(&json);
    } else {
        assert!(response.status.as_u16() == 401 || response.status.as_u16() == 404);
    }
}

