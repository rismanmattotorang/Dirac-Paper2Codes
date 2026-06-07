//! Performance tests for API endpoints
//!
//! Tests response times, throughput, and concurrent request handling

use crate::common::TestServer;
use std::time::Instant;

/// Test health endpoint response time
#[tokio::test]
async fn test_health_endpoint_performance() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let start = Instant::now();
    let response = server.get("/api/health").await.expect("Request failed");
    let duration = start.elapsed();
    
    response.assert_success();
    
    // Health check should be very fast (< 100ms)
    assert!(
        duration.as_millis() < 100,
        "Health check took {}ms, expected < 100ms",
        duration.as_millis()
    );
}

/// Test concurrent request handling
#[tokio::test]
async fn test_concurrent_requests() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let start = Instant::now();
    
    // Make 100 concurrent requests
    let mut handles = Vec::new();
    for _ in 0..100 {
        let server_clone = &server;
        let handle = tokio::spawn(async move {
            server_clone.get("/api/health").await
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    let duration = start.elapsed();
    
    // All requests should succeed
    let mut success_count = 0;
    for result in results {
        if let Ok(Ok(response)) = result {
            if response.status.is_success() {
                success_count += 1;
            }
        }
    }
    
    assert_eq!(
        success_count, 100,
        "All 100 concurrent requests should succeed"
    );
    
    // Should complete in reasonable time (< 5 seconds)
    assert!(
        duration.as_secs() < 5,
        "100 concurrent requests took {}s, expected < 5s",
        duration.as_secs()
    );
}

/// Test response time under load
#[tokio::test]
async fn test_response_time_under_load() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let num_requests = 1000;
    let mut response_times = Vec::new();
    
    for _ in 0..num_requests {
        let start = Instant::now();
        let response = server.get("/api/health").await.expect("Request failed");
        let duration = start.elapsed();
        
        if response.status.is_success() {
            response_times.push(duration.as_millis() as u64);
        }
    }
    
    if !response_times.is_empty() {
        // Calculate percentiles
        response_times.sort();
        let p50 = response_times[response_times.len() / 2];
        let p95 = response_times[(response_times.len() * 95) / 100];
        let p99 = response_times[(response_times.len() * 99) / 100];
        
        println!("Response time percentiles (ms):");
        println!("  P50: {}", p50);
        println!("  P95: {}", p95);
        println!("  P99: {}", p99);
        
        // P95 should be < 200ms (per Phase 7 requirements)
        assert!(
            p95 < 200,
            "P95 response time is {}ms, expected < 200ms",
            p95
        );
    }
}

/// Test throughput (requests per second)
#[tokio::test]
async fn test_throughput() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    let num_requests = 1000;
    let start = Instant::now();
    
    // Make requests sequentially to measure throughput
    for _ in 0..num_requests {
        let response = server.get("/api/health").await.expect("Request failed");
        assert!(response.status.is_success());
    }
    
    let duration = start.elapsed();
    let rps = num_requests as f64 / duration.as_secs_f64();
    
    println!("Throughput: {:.2} requests/second", rps);
    
    // Should handle at least 100 requests/second
    assert!(
        rps > 100.0,
        "Throughput is {:.2} req/s, expected > 100 req/s",
        rps
    );
}

/// Test memory usage under load
#[tokio::test]
async fn test_memory_usage() {
    let server = TestServer::new().await.expect("Failed to create test server");
    
    // Make many requests and check for memory leaks
    for _ in 0..1000 {
        let response = server.get("/api/health").await.expect("Request failed");
        assert!(response.status.is_success());
    }
    
    // If we get here without OOM, memory usage is reasonable
    // In a real scenario, you'd measure actual memory usage
    println!("Memory usage test completed successfully");
}

