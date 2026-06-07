use crate::api::state::AppState;
use crate::api::types::responses::{ApiResponse, HealthResponse};
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::sync::Arc;

/// Health check endpoint
///
/// Returns the health status of the API server
/// GET /api/health
pub async fn health_check(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<HealthResponse>>, (StatusCode, Json<serde_json::Value>)> {
    // Extract request ID from headers (set by middleware)
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    // Check storage connection if enabled
    let storage_healthy = if state.config.storage.enabled {
        let storage = state.storage.read().await;
        if let Some(storage_manager) = storage.as_ref() {
            // Check connection asynchronously
            storage_manager.is_connected().await
        } else {
            false
        }
    } else {
        true // Storage not required
    };

    let status = if storage_healthy {
        "healthy".to_string()
    } else {
        "degraded".to_string()
    };

    let response = ApiResponse::with_request_id(
        HealthResponse {
            status,
            version: crate::VERSION.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
        request_id,
    );

    Ok(Json(response))
}
