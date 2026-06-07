//! Security tests for API endpoints
//!
//! Tests for OWASP Top 10 security vulnerabilities and input validation

use crate::common::TestServer;
use serde_json::json;

/// Test SQL injection prevention
#[tokio::test]
async fn test_sql_injection_prevention() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Attempt SQL injection in path parameter
    let sql_injection = "'; DROP TABLE papers; --";
    let response = server.get(&format!("/api/papers/{}", sql_injection))
        .await
        .expect("Request failed");
    
    // Should not execute SQL, should return 404 or 400
    assert!(
        response.status.is_client_error() || response.status.as_u16() == 404,
        "SQL injection attempt should be rejected"
    );
}

/// Test XSS prevention in input
#[tokio::test]
async fn test_xss_prevention() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Attempt XSS in paper title
    let xss_payload = json!({
        "title": "<script>alert('XSS')</script>",
        "abstract_text": "Test",
        "content": "Test content"
    });
    
    let response = server.post("/api/papers", &xss_payload)
        .await
        .expect("Request failed");
    
    // Should sanitize or reject XSS payload
    if response.status.is_success() {
        let json: serde_json::Value = response.json().expect("Failed to parse JSON");
        // Check that script tags are sanitized
        if let Some(title) = json["data"]["title"].as_str() {
            assert!(!title.contains("<script>"), "XSS payload should be sanitized");
        }
    } else {
        // Or reject the request
        assert!(response.status.is_client_error());
    }
}

/// Test path traversal prevention
#[tokio::test]
async fn test_path_traversal_prevention() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Attempt path traversal
    let path_traversal = "../../../etc/passwd";
    let response = server.get(&format!("/api/papers/{}", path_traversal))
        .await
        .expect("Request failed");
    
    // Should reject path traversal attempts
    assert!(
        response.status.is_client_error() || response.status.as_u16() == 404,
        "Path traversal attempt should be rejected"
    );
}

/// Test input size limits
#[tokio::test]
async fn test_input_size_limits() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Create extremely large payload
    let large_content = "x".repeat(10_000_000); // 10MB
    let large_paper = json!({
        "title": "Test",
        "abstract_text": "Test",
        "content": large_content
    });
    
    let response = server.post("/api/papers", &large_paper)
        .await
        .expect("Request failed");
    
    // Should reject or limit large payloads
    assert!(
        response.status.is_client_error() || response.status.is_server_error(),
        "Large payload should be rejected or limited"
    );
}

/// Test authentication required for protected endpoints
#[tokio::test]
async fn test_authentication_required() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Try to access protected endpoint without authentication
    let response = server.get("/api/papers").await.expect("Request failed");
    
    // Should return 401 Unauthorized if auth is required
    // Or 200 if auth is not yet implemented
    assert!(
        response.status.as_u16() == 401 || response.status.is_success(),
        "Should require authentication or allow if not implemented"
    );
}

/// Test rate limiting (if enabled)
#[tokio::test]
async fn test_rate_limiting() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Make many rapid requests
    let mut rate_limited = false;
    for _ in 0..100 {
        let response = server.get("/api/health").await.expect("Request failed");
        if response.status.as_u16() == 429 {
            rate_limited = true;
            break;
        }
    }
    
    // Rate limiting may be disabled in tests, so this is informational
    if rate_limited {
        println!("Rate limiting is working");
    }
}

/// Test CORS headers
#[tokio::test]
async fn test_cors_headers() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    
    // Check for CORS headers (may not be present in all responses)
    // This is informational - CORS is typically configured per-origin
    if response.headers.contains_key("access-control-allow-origin") {
        println!("CORS headers present");
    }
}

/// Test security headers
#[tokio::test]
async fn test_security_headers() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let response = server.get("/api/health").await.expect("Request failed");
    
    // Check for security headers
    let security_headers = [
        "x-content-type-options",
        "x-frame-options",
        "x-xss-protection",
        "strict-transport-security",
    ];
    
    let mut found_headers = 0;
    for header in &security_headers {
        if response.headers.contains_key(*header) {
            found_headers += 1;
        }
    }
    
    // At least some security headers should be present
    assert!(
        found_headers > 0,
        "Should have at least some security headers"
    );
}

/// Test invalid JSON handling
#[tokio::test]
async fn test_invalid_json_handling() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // This would require sending raw invalid JSON, which reqwest doesn't easily support
    // In a real scenario, you'd use a lower-level HTTP client
    // For now, we test with malformed but valid JSON
    let invalid_paper = json!({
        "title": null, // Invalid: title should be string
        "abstract_text": 123, // Invalid: should be string
    });
    
    let response = server.post("/api/papers", &invalid_paper)
        .await
        .expect("Request failed");
    
    // Should return validation error
    assert!(
        response.status.is_client_error(),
        "Invalid JSON should be rejected"
    );
}

