use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

use paper2codes::api::server::ApiServer;
use paper2codes::config::Config;

#[tokio::test]
async fn test_get_settings() {
    // Create test config
    let config = Config::default();
    
    // Create API server
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    // Make request
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Check response
    assert_eq!(response.status(), StatusCode::OK);
    
    // Parse body
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let settings: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify structure
    assert!(settings.get("general").is_some());
    assert!(settings.get("security").is_some());
    assert!(settings.get("notifications").is_some());
    assert!(settings.get("llm").is_some());
    assert!(settings.get("database").is_some());
    assert!(settings.get("team").is_some());
}

#[tokio::test]
async fn test_update_settings() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let update_request = json!({
        "general": {
            "organization_name": "Test Organization",
            "default_domain": "Deep Learning",
            "dark_mode": true,
            "theme": "dark"
        },
        "notifications": {
            "paper_processing_complete": true,
            "code_generation_errors": false,
            "task_queue_updates": true,
            "weekly_report": false
        }
    });
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .method("PUT")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_settings_invalid_temperature() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let update_request = json!({
        "llm": {
            "temperature": 3.0  // Invalid: > 2.0
        }
    });
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .method("PUT")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_enable_2fa() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/security/2fa/enable")
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(result.get("qr_code").is_some());
    assert!(result.get("secret").is_some());
    assert!(result.get("backup_codes").is_some());
}

#[tokio::test]
async fn test_disable_2fa() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/security/2fa/disable")
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_revoke_session() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/security/sessions/session_123")
                .method("DELETE")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_database_connection_test() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/database/test")
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert!(result.get("success").is_some());
    assert!(result.get("message").is_some());
}

#[tokio::test]
async fn test_add_team_member() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let add_request = json!({
        "name": "John Doe",
        "email": "john@example.com",
        "role": "Editor"
    });
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/team/members")
                .method("POST")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&add_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let member: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(member["name"], "John Doe");
    assert_eq!(member["email"], "john@example.com");
    assert_eq!(member["role"], "Editor");
    assert_eq!(member["status"], "Invited");
}

#[tokio::test]
async fn test_remove_team_member() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/team/members/user_123")
                .method("DELETE")
                .header("Authorization", "Bearer test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_update_team_member_role() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    let update_request = json!({
        "role": "Admin"
    });
    
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings/team/members/user_123/role")
                .method("PUT")
                .header("Authorization", "Bearer test-token")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let member: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(member["role"], "Admin");
}

#[tokio::test]
async fn test_settings_requires_auth() {
    let config = Config::default();
    let server = ApiServer::new(config).await.expect("Failed to create server");
    
    // Request without auth header
    let response = server
        .router
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    // Should return 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

