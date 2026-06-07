//! Helper functions for API integration tests

use serde_json::Value;
use std::collections::HashMap;

/// Create authentication headers for test requests
pub fn auth_headers(token: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers
}

/// Extract error message from API error response
pub fn extract_error_message(response: &Value) -> Option<String> {
    response
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

/// Extract data from API success response
pub fn extract_data<T: serde::de::DeserializeOwned>(response: &Value) -> Option<T> {
    response
        .get("data")
        .and_then(|d| serde_json::from_value(d.clone()).ok())
}

/// Create a test paper payload
pub fn create_test_paper() -> Value {
    serde_json::json!({
        "title": "Test Paper",
        "abstract_text": "This is a test paper abstract",
        "content": "Test paper content",
        "metadata": {
            "authors": ["Test Author"],
            "year": 2024
        }
    })
}

/// Create a test repository payload
pub fn create_test_repository() -> Value {
    serde_json::json!({
        "name": "test-repo",
        "description": "Test repository",
        "language": "python"
    })
}

/// Create a test task payload
pub fn create_test_task() -> Value {
    serde_json::json!({
        "task_type": "planning",
        "description": "Test task",
        "paper_id": "test-paper-id"
    })
}

/// Wait for async operation to complete
pub async fn wait_for_condition<F, Fut>(mut condition: F, timeout_ms: u64) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    false
}

/// Generate a random test ID
pub fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Assert JSON response structure
pub fn assert_api_response(response: &Value) {
    assert!(
        response.get("success").is_some(),
        "Response should have 'success' field"
    );
    assert!(
        response.get("data").is_some() || response.get("error").is_some(),
        "Response should have either 'data' or 'error' field"
    );
    assert!(
        response.get("meta").is_some(),
        "Response should have 'meta' field"
    );
}

