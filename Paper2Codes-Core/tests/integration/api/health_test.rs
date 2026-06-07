//! Integration tests for health check endpoint

use crate::common::TestServer;
use crate::common::helpers::assert_api_response;

#[tokio::test]
async fn test_health_check() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    let json: serde_json::Value = response.json().expect("Failed to parse JSON");
    assert_api_response(&json);
    
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "healthy");
    assert!(json["data"]["version"].is_string());
    assert!(json["data"]["timestamp"].is_string());
}

#[tokio::test]
async fn test_health_check_has_request_id() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    // Check that request ID is in response headers
    assert!(response.headers.contains_key("x-request-id"));
}

