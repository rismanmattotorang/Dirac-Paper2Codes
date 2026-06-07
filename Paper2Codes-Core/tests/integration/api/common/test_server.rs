//! Test server helper for integration tests
//!
//! Provides a test server instance with in-memory storage and test fixtures

use axum::Router;
use paper2codes::api::ApiServer;
use paper2codes::config::Config;
use std::net::SocketAddr;
use tower::ServiceExt;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;

/// Test server for integration tests
pub struct TestServer {
    pub addr: SocketAddr,
    pub app: Router,
    pub config: Config,
    pub client: reqwest::Client,
}

impl TestServer {
    /// Create a new test server with in-memory storage
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create test configuration
        let mut config = Config::default();
        
        // Use in-memory storage for tests
        config.storage.enabled = true;
        config.storage.connection_string = "memory".to_string();
        config.storage.namespace = "test".to_string();
        config.storage.database = "test".to_string();
        
        // Disable rate limiting for tests if the field exists
        // Note: Rate limiting may be controlled differently in actual implementation
        
        // Create API server
        let api_server = ApiServer::new(config.clone()).await?;
        
        // Extract router
        let router = api_server.router;
        
        // Bind to a random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        
        // Spawn server in background
        let server_router = router.clone();
        tokio::spawn(async move {
            axum::serve(listener, server_router).await.unwrap();
        });
        
        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(Self {
            addr,
            app: router,
            config,
            client,
        })
    }
    
    /// Get the base URL for the test server
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
    
    /// Make a GET request to the test server
    pub async fn get(&self, path: &str) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("GET", path, None::<Value>, None).await
    }
    
    /// Make a GET request with headers
    pub async fn get_with_headers(
        &self,
        path: &str,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("GET", path, None::<Value>, Some(headers)).await
    }
    
    /// Make a POST request to the test server
    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: T,
    ) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("POST", path, Some(body), None).await
    }
    
    /// Make a POST request with headers
    pub async fn post_with_headers<T: serde::Serialize>(
        &self,
        path: &str,
        body: T,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("POST", path, Some(body), Some(headers)).await
    }
    
    /// Make a PUT request to the test server
    pub async fn put<T: serde::Serialize>(
        &self,
        path: &str,
        body: T,
    ) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("PUT", path, Some(body), None).await
    }
    
    /// Make a DELETE request to the test server
    pub async fn delete(&self, path: &str) -> Result<TestResponse, Box<dyn std::error::Error>> {
        self.request("DELETE", path, None::<Value>, None).await
    }
    
    /// Make a request to the test server
    async fn request<T: serde::Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<T>,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> Result<TestResponse, Box<dyn std::error::Error>> {
        let url = format!("http://{}{}", self.addr, path);
        let mut request_builder = self.client.request(
            method.parse()?,
            &url,
        );
        
        // Add headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request_builder = request_builder.header(&key, value);
            }
        }
        
        // Add body if present
        if let Some(body) = body {
            request_builder = request_builder.json(&body);
        }
        
        let response = request_builder.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let body_text = response.text().await?;
        
        Ok(TestResponse {
            status,
            headers,
            body: body_text,
        })
    }
}

/// Test response wrapper
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

impl TestResponse {
    /// Assert the status code
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}: {}",
            expected, self.status, self.body
        );
        self
    }
    
    /// Parse the response body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.body)
    }
    
    /// Get the response body as text
    pub fn text(&self) -> &str {
        &self.body
    }
    
    /// Assert the response is successful (2xx)
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.status.is_success(),
            "Expected success status, got {}: {}",
            self.status,
            self.body
        );
        self
    }
    
    /// Assert the response is a client error (4xx)
    pub fn assert_client_error(&self) -> &Self {
        assert!(
            self.status.is_client_error(),
            "Expected client error status, got {}: {}",
            self.status,
            self.body
        );
        self
    }
    
    /// Assert the response is a server error (5xx)
    pub fn assert_server_error(&self) -> &Self {
        assert!(
            self.status.is_server_error(),
            "Expected server error status, got {}: {}",
            self.status,
            self.body
        );
        self
    }
}

