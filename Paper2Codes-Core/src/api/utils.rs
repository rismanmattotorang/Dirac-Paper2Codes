//! Shared utilities for API handlers

pub mod batching;

pub use batching::{BatchItem, BatchItemResponse, BatchRequest, BatchResponse};

use crate::api::types::errors::ApiError;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use uuid::Uuid;

/// Extract request ID from headers or generate a new one
pub fn get_request_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// Create standardized error response
pub fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
    request_id: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let request_id = request_id.into();
    let error = ApiError::new(code, message);
    let mut error_value = serde_json::to_value(error).unwrap_or_else(|_| {
        serde_json::json!({
            "success": false,
            "error": {"code": code, "message": message}
        })
    });

    // Update request ID in error response meta
    if let Some(meta) = error_value.get_mut("meta") {
        if let Some(meta_obj) = meta.as_object_mut() {
            meta_obj.insert(
                "request_id".to_string(),
                serde_json::Value::String(request_id),
            );
        }
    }

    (status, Json(error_value))
}

/// Create validation error response
pub fn validation_error_response(
    errors: &validator::ValidationErrors,
    request_id: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let request_id = request_id.into();
    let error_messages: Vec<String> = errors
        .field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().map(move |e| {
                let message = e
                    .message
                    .as_ref()
                    .map(|m| m.as_ref())
                    .unwrap_or_else(|| e.code.as_ref());
                format!("{}: {}", field, message)
            })
        })
        .collect();

    let error = ApiError::with_details(
        "VALIDATION_ERROR",
        "Request validation failed",
        serde_json::json!({
            "errors": error_messages
        }),
    );

    let mut error_value = serde_json::to_value(error).unwrap_or_else(|_| {
        serde_json::json!({
            "success": false,
            "error": {"code": "VALIDATION_ERROR", "message": "Request validation failed"}
        })
    });

    // Update request ID in error response meta
    if let Some(meta) = error_value.get_mut("meta") {
        if let Some(meta_obj) = meta.as_object_mut() {
            meta_obj.insert(
                "request_id".to_string(),
                serde_json::Value::String(request_id),
            );
        }
    }

    (StatusCode::BAD_REQUEST, Json(error_value))
}
