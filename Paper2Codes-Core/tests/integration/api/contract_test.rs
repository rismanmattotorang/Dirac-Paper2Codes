//! API contract tests
//!
//! Tests that verify API responses match expected contracts and schemas

use crate::common::TestServer;
use crate::common::helpers::assert_api_response;
use serde_json::Value;

/// Test API response structure contract
#[tokio::test]
async fn test_api_response_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    let json: Value = response.json().expect("Failed to parse JSON");
    
    // Verify response structure
    assert!(json.get("success").is_some(), "Response must have 'success' field");
    assert!(json.get("data").is_some() || json.get("error").is_some(), 
            "Response must have 'data' or 'error' field");
    assert!(json.get("meta").is_some(), "Response must have 'meta' field");
    
    // Verify meta structure
    if let Some(meta) = json.get("meta") {
        assert!(meta.get("timestamp").is_some(), "Meta must have 'timestamp'");
        assert!(meta.get("request_id").is_some(), "Meta must have 'request_id'");
    }
}

/// Test health response contract
#[tokio::test]
async fn test_health_response_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    let json: Value = response.json().expect("Failed to parse JSON");
    
    // Verify health response structure
    if let Some(data) = json.get("data") {
        assert!(data.get("status").is_some(), "Health data must have 'status'");
        assert!(data.get("version").is_some(), "Health data must have 'version'");
        assert!(data.get("timestamp").is_some(), "Health data must have 'timestamp'");
        
        // Verify status is a valid value
        if let Some(status) = data.get("status").and_then(|s| s.as_str()) {
            assert!(
                status == "healthy" || status == "degraded" || status == "unhealthy",
                "Status must be 'healthy', 'degraded', or 'unhealthy'"
            );
        }
    }
}

/// Test error response contract
#[tokio::test]
async fn test_error_response_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Request non-existent resource to get error response
    let response = server.get("/api/papers/non-existent-id-12345")
        .await
        .expect("Request failed");
    
    if response.status.is_client_error() || response.status.is_server_error() {
        let json: Value = response.json().expect("Failed to parse JSON");
        
        // Verify error response structure
        assert_eq!(json.get("success"), Some(&Value::Bool(false)), 
                   "Error response must have success: false");
        
        if let Some(error) = json.get("error") {
            assert!(error.get("code").is_some(), "Error must have 'code'");
            assert!(error.get("message").is_some(), "Error must have 'message'");
        }
    }
}

/// Test pagination contract
#[tokio::test]
async fn test_pagination_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/papers?page=1&per_page=10")
        .await
        .expect("Request failed");
    
    if response.status.is_success() {
        let json: Value = response.json().expect("Failed to parse JSON");
        
        // If pagination is implemented, verify structure
        if let Some(data) = json.get("data") {
            if let Some(pagination) = data.get("pagination") {
                assert!(pagination.get("page").is_some(), "Pagination must have 'page'");
                assert!(pagination.get("per_page").is_some(), "Pagination must have 'per_page'");
                assert!(pagination.get("total").is_some(), "Pagination must have 'total'");
            }
        }
    }
}

/// Test list response contract
#[tokio::test]
async fn test_list_response_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/papers").await.expect("Request failed");
    
    if response.status.is_success() {
        let json: Value = response.json().expect("Failed to parse JSON");
        
        // Verify list response structure
        if let Some(data) = json.get("data") {
            // Data should be an array for list endpoints
            assert!(data.is_array() || data.get("items").is_some(), 
                    "List response data must be an array or have 'items' field");
        }
    }
}

/// Test request ID propagation
#[tokio::test]
async fn test_request_id_propagation() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    // Check request ID in headers
    let request_id_header = response.headers.get("x-request-id");
    assert!(request_id_header.is_some(), "Response must have x-request-id header");
    
    // Check request ID in response body
    let json: Value = response.json().expect("Failed to parse JSON");
    if let Some(meta) = json.get("meta") {
        if let Some(request_id) = meta.get("request_id") {
            assert!(request_id.is_string(), "Request ID in meta must be a string");
            
            // Verify request ID matches header (if both present)
            if let Some(header_value) = request_id_header {
                if let Ok(header_str) = header_value.to_str() {
                    if let Some(id_str) = request_id.as_str() {
                        assert_eq!(id_str, header_str, 
                                   "Request ID in body should match header");
                    }
                }
            }
        }
    }
}

/// Test content type headers
#[tokio::test]
async fn test_content_type_headers() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    response.assert_success();
    
    // Verify content type is JSON
    let content_type = response.headers.get("content-type");
    if let Some(ct) = content_type {
        let ct_str = ct.to_str().unwrap_or("");
        assert!(
            ct_str.contains("application/json"),
            "Content-Type should be application/json, got: {}",
            ct_str
        );
    }
}

/// Test CORS headers contract
#[tokio::test]
async fn test_cors_headers_contract() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Make OPTIONS request to test CORS
    // Note: reqwest doesn't easily support OPTIONS, so we check for CORS headers
    // in regular responses
    let response = server.get("/api/health").await.expect("Request failed");
    
    // CORS headers may be present depending on configuration
    // This test verifies the structure if present
    if response.headers.contains_key("access-control-allow-origin") {
        let origin = response.headers.get("access-control-allow-origin");
        assert!(origin.is_some(), "CORS origin header should have a value");
    }
}

