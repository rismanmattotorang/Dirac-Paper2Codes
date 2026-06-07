//! Batch request handler
//!
//! Handles batched API requests for improved performance

use crate::api::handlers::{papers::get_paper, repositories::get_repository, tasks::get_task};
use crate::api::state::AppState;
use crate::api::types::responses::ApiResponse;
use crate::api::utils::{
    error_response, get_request_id, BatchItemResponse, BatchRequest, BatchResponse,
};
use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::Json,
};
use std::sync::Arc;
use tracing::info;

/// Process a batch of API requests
///
/// POST /api/batch
pub async fn process_batch(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(batch_request): Json<BatchRequest>,
) -> Result<Json<ApiResponse<BatchResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    info!(request_id = %request_id, count = batch_request.requests.len(), "Processing batch request");

    // Validate batch request
    if let Err(validation_err) = batch_request.validate() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            &validation_err,
            request_id,
        ));
    }

    // Process each request in the batch
    let mut batch_response = BatchResponse::new();

    for item in batch_request.requests {
        let result = process_batch_item(&state, &item, &headers).await;
        batch_response.add_result(item.id, result);
    }

    info!(request_id = %request_id, "Batch request completed");

    Ok(Json(ApiResponse::with_request_id(
        batch_response,
        request_id,
    )))
}

/// Process a single batch item
async fn process_batch_item(
    state: &Arc<AppState>,
    item: &crate::api::utils::BatchItem,
    headers: &HeaderMap,
) -> BatchItemResponse {
    // Route to appropriate handler based on path
    // This is a simplified implementation - in production, you'd want
    // a more sophisticated routing mechanism

    match item.method.as_str() {
        "GET" => {
            if item.path.starts_with("/api/papers/") {
                // Extract ID from path
                if let Some(id) = item.path.strip_prefix("/api/papers/") {
                    match get_paper(
                        Extension(state.clone()),
                        axum::extract::Path(id.to_string()),
                        headers.clone(),
                    )
                    .await
                    {
                        Ok(json_response) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_response.0) {
                                return BatchItemResponse {
                                    status: 200,
                                    body: json_value,
                                    headers: None,
                                    error: None,
                                };
                            }
                        }
                        Err((status, json_error)) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_error.0) {
                                return BatchItemResponse {
                                    status: status.as_u16(),
                                    body: json_value,
                                    headers: None,
                                    error: Some("Request failed".to_string()),
                                };
                            }
                        }
                    }
                }
            } else if item.path.starts_with("/api/repositories/") {
                // Similar handling for repositories
                if let Some(id) = item.path.strip_prefix("/api/repositories/") {
                    match get_repository(
                        Extension(state.clone()),
                        axum::extract::Path(id.to_string()),
                        headers.clone(),
                    )
                    .await
                    {
                        Ok(json_response) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_response.0) {
                                return BatchItemResponse {
                                    status: 200,
                                    body: json_value,
                                    headers: None,
                                    error: None,
                                };
                            }
                        }
                        Err((status, json_error)) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_error.0) {
                                return BatchItemResponse {
                                    status: status.as_u16(),
                                    body: json_value,
                                    headers: None,
                                    error: Some("Request failed".to_string()),
                                };
                            }
                        }
                    }
                }
            } else if item.path.starts_with("/api/tasks/") {
                // Similar handling for tasks
                if let Some(id) = item.path.strip_prefix("/api/tasks/") {
                    match get_task(
                        Extension(state.clone()),
                        axum::extract::Path(id.to_string()),
                        headers.clone(),
                    )
                    .await
                    {
                        Ok(json_response) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_response.0) {
                                return BatchItemResponse {
                                    status: 200,
                                    body: json_value,
                                    headers: None,
                                    error: None,
                                };
                            }
                        }
                        Err((status, json_error)) => {
                            // Extract JSON value from Json wrapper
                            if let Ok(json_value) = serde_json::to_value(&json_error.0) {
                                return BatchItemResponse {
                                    status: status.as_u16(),
                                    body: json_value,
                                    headers: None,
                                    error: Some("Request failed".to_string()),
                                };
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Unsupported method
            return BatchItemResponse {
                status: 405,
                body: serde_json::json!({
                    "error": "Method not supported in batch requests"
                }),
                headers: None,
                error: Some("Method not supported".to_string()),
            };
        }
    }

    // Default error response
    BatchItemResponse {
        status: 404,
        body: serde_json::json!({
            "error": "Path not found or not supported in batch requests"
        }),
        headers: None,
        error: Some("Path not found".to_string()),
    }
}
