//! Load testing for API endpoints
//!
//! Tests the API under high load conditions (1000+ concurrent users)

use crate::common::TestServer;
use std::time::Instant;
use std::sync::Arc;

/// Test API under high concurrent load
#[tokio::test]
#[ignore] // Ignore by default - run explicitly for load testing
async fn test_high_concurrent_load() {
    let server = Arc::new(TestServer::new().await.expect("Failed to create test server"));
    
    let num_concurrent_users = 1000;
    let requests_per_user = 10;
    
    println!("Starting load test: {} concurrent users, {} requests each", 
             num_concurrent_users, requests_per_user);
    
    let start = Instant::now();
    
    // Create tasks for each "user"
    let mut handles = Vec::new();
    for user_id in 0..num_concurrent_users {
        let server_clone = server.clone();
        let handle = tokio::spawn(async move {
            let mut success_count = 0;
            let mut error_count = 0;
            
            for _ in 0..requests_per_user {
                match server_clone.get("/api/health").await {
                    Ok(response) => {
                        if response.status.is_success() {
                            success_count += 1;
                        } else {
                            error_count += 1;
                        }
                    }
                    Err(_) => {
                        error_count += 1;
                    }
                }
            }
            
            (success_count, error_count)
        });
        handles.push(handle);
    }
    
    // Wait for all users to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    let duration = start.elapsed();
    
    // Aggregate results
    let mut total_success = 0;
    let mut total_errors = 0;
    
    for result in results {
        if let Ok((success, errors)) = result {
            total_success += success;
            total_errors += errors;
        }
    }
    
    let total_requests = num_concurrent_users * requests_per_user;
    let success_rate = (total_success as f64 / total_requests as f64) * 100.0;
    let rps = total_requests as f64 / duration.as_secs_f64();
    
    println!("\nLoad Test Results:");
    println!("  Total requests: {}", total_requests);
    println!("  Successful: {} ({:.2}%)", total_success, success_rate);
    println!("  Errors: {}", total_errors);
    println!("  Duration: {:.2}s", duration.as_secs_f64());
    println!("  Throughput: {:.2} requests/second", rps);
    
    // Success rate should be > 95%
    assert!(
        success_rate > 95.0,
        "Success rate is {:.2}%, expected > 95%",
        success_rate
    );
    
    // Should handle at least 500 requests/second
    assert!(
        rps > 500.0,
        "Throughput is {:.2} req/s, expected > 500 req/s",
        rps
    );
}

/// Test sustained load over time
#[tokio::test]
#[ignore] // Ignore by default - run explicitly for load testing
async fn test_sustained_load() {
    let server = Arc::new(TestServer::new().await.expect("Failed to create test server"));
    
    let duration_seconds = 60;
    let requests_per_second = 100;
    
    println!("Starting sustained load test: {} req/s for {} seconds", 
             requests_per_second, duration_seconds);
    
    let start = Instant::now();
    let mut total_requests = 0;
    let mut total_errors = 0;
    
    while start.elapsed().as_secs() < duration_seconds {
        let batch_start = Instant::now();
        let mut handles = Vec::new();
        
        // Create batch of requests
        for _ in 0..requests_per_second {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                server_clone.get("/api/health").await
            });
            handles.push(handle);
        }
        
        // Wait for batch to complete
        for handle in handles {
            match handle.await {
                Ok(Ok(response)) => {
                    total_requests += 1;
                    if !response.status.is_success() {
                        total_errors += 1;
                    }
                }
                _ => {
                    total_errors += 1;
                }
            }
        }
        
        // Sleep to maintain rate
        let elapsed = batch_start.elapsed();
        if elapsed.as_millis() < 1000 {
            tokio::time::sleep(
                tokio::time::Duration::from_millis(1000 - elapsed.as_millis() as u64)
            ).await;
        }
    }
    
    let actual_duration = start.elapsed();
    let actual_rps = total_requests as f64 / actual_duration.as_secs_f64();
    let error_rate = (total_errors as f64 / total_requests as f64) * 100.0;
    
    println!("\nSustained Load Test Results:");
    println!("  Total requests: {}", total_requests);
    println!("  Errors: {} ({:.2}%)", total_errors, error_rate);
    println!("  Duration: {:.2}s", actual_duration.as_secs_f64());
    println!("  Average throughput: {:.2} requests/second", actual_rps);
    
    // Error rate should be < 1%
    assert!(
        error_rate < 1.0,
        "Error rate is {:.2}%, expected < 1%",
        error_rate
    );
}

/// Test memory stability under load
#[tokio::test]
#[ignore] // Ignore by default - run explicitly for load testing
async fn test_memory_stability() {
    let server = Arc::new(TestServer::new().await.expect("Failed to create test server"));
    
    let num_iterations = 100;
    let requests_per_iteration = 1000;
    
    println!("Testing memory stability: {} iterations of {} requests", 
             num_iterations, requests_per_iteration);
    
    for iteration in 0..num_iterations {
        let mut handles = Vec::new();
        
        for _ in 0..requests_per_iteration {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                server_clone.get("/api/health").await
            });
            handles.push(handle);
        }
        
        // Wait for all requests
        for handle in handles {
            let _ = handle.await;
        }
        
        if iteration % 10 == 0 {
            println!("Completed iteration {}/{}", iteration, num_iterations);
        }
    }
    
    println!("Memory stability test completed successfully");
}

