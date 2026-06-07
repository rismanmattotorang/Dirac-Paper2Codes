use crate::api::types::errors::ApiError;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use tracing::error;

/// Global error handler middleware
pub async fn error_handler(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let response = next.run(request).await;

    // Log errors if status is server error
    if response.status().is_server_error() {
        error!(
            uri = %uri,
            method = %method,
            status = %response.status().as_u16(),
            "Server error occurred"
        );
    }

    // Convert 401/403 errors to proper JSON format if they're plain text
    if response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::FORBIDDEN {
        // Check if response body is text/plain (string error)
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok());

        if content_type
            .map(|ct| ct.contains("text/plain"))
            .unwrap_or(true)
        {
            // Extract error message from response
            let (mut parts, body) = response.into_parts();

            // Try to read body as string (non-blocking check)
            let error_message = if let Ok(body_bytes) = axum::body::to_bytes(body, usize::MAX).await
            {
                String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| {
                    if parts.status == StatusCode::UNAUTHORIZED {
                        "Unauthorized".to_string()
                    } else {
                        "Forbidden".to_string()
                    }
                })
            } else {
                if parts.status == StatusCode::UNAUTHORIZED {
                    "Unauthorized".to_string()
                } else {
                    "Forbidden".to_string()
                }
            };

            let api_error = ApiError::new(
                if parts.status == StatusCode::UNAUTHORIZED {
                    "UNAUTHORIZED"
                } else {
                    "FORBIDDEN"
                },
                &error_message,
            );

            // Set proper content type
            parts.headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/json"),
            );

            let json_body = serde_json::to_string(&api_error).unwrap_or_else(|_| {
                serde_json::json!({
                    "success": false,
                    "error": {
                        "code": if parts.status == StatusCode::UNAUTHORIZED { "UNAUTHORIZED" } else { "FORBIDDEN" },
                        "message": error_message
                    }
                }).to_string()
            });

            return Response::from_parts(parts, axum::body::Body::from(json_body));
        }
    }

    response
}
