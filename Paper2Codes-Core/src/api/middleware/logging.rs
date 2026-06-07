use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
use tracing::{error, info, Span};

/// Middleware for logging HTTP requests and responses
pub async fn log_requests(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Extract request ID from headers
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Set request ID in tracing span for correlation
    Span::current().record("request_id", &request_id.as_str());

    // Process request
    let response = next.run(request).await;
    let duration = start.elapsed();

    let status = response.status();

    // Log the request with appropriate level
    if status.is_server_error() {
        error!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = duration.as_millis(),
            request_id = %request_id,
            "Request completed with server error"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = duration.as_millis(),
            request_id = %request_id,
            "Request completed with client error"
        );
    } else {
        info!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = duration.as_millis(),
            request_id = %request_id,
            "Request completed"
        );
    }

    response
}
