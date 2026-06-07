//! Request batching utilities
//!
//! Provides utilities for batching multiple API requests into a single request
//! to reduce network overhead and improve performance

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Batch request for multiple operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    /// List of individual requests to batch
    pub requests: Vec<BatchItem>,
}

/// Individual item in a batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    /// Unique ID for this request in the batch
    pub id: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request body (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Request headers (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Batch response containing results for all batched requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Results for each batched request, keyed by request ID
    pub results: HashMap<String, BatchItemResponse>,
}

/// Response for a single batched request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body
    pub body: serde_json::Value,
    /// Response headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Error message if request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchRequest {
    /// Create a new batch request
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Add a request to the batch
    pub fn add(&mut self, id: String, method: String, path: String) -> &mut Self {
        self.requests.push(BatchItem {
            id,
            method,
            path,
            body: None,
            headers: None,
        });
        self
    }

    /// Add a request with body
    pub fn add_with_body(
        &mut self,
        id: String,
        method: String,
        path: String,
        body: serde_json::Value,
    ) -> &mut Self {
        self.requests.push(BatchItem {
            id,
            method,
            path,
            body: Some(body),
            headers: None,
        });
        self
    }

    /// Validate batch request
    pub fn validate(&self) -> Result<(), String> {
        if self.requests.is_empty() {
            return Err("Batch request must contain at least one request".to_string());
        }

        if self.requests.len() > 100 {
            return Err("Batch request cannot contain more than 100 requests".to_string());
        }

        // Check for duplicate IDs
        let mut ids = std::collections::HashSet::new();
        for item in &self.requests {
            if !ids.insert(&item.id) {
                return Err(format!("Duplicate request ID: {}", item.id));
            }
        }

        Ok(())
    }
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchResponse {
    /// Create a new batch response
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    /// Add a result to the batch response
    pub fn add_result(&mut self, id: String, response: BatchItemResponse) {
        self.results.insert(id, response);
    }

    /// Get a result by ID
    pub fn get_result(&self, id: &str) -> Option<&BatchItemResponse> {
        self.results.get(id)
    }
}

impl Default for BatchResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_validation() {
        let mut batch = BatchRequest::new();

        // Empty batch should fail
        assert!(batch.validate().is_err());

        // Valid batch
        batch.add(
            "1".to_string(),
            "GET".to_string(),
            "/api/papers/1".to_string(),
        );
        assert!(batch.validate().is_ok());

        // Duplicate IDs should fail
        batch.add(
            "1".to_string(),
            "GET".to_string(),
            "/api/papers/2".to_string(),
        );
        assert!(batch.validate().is_err());
    }
}
