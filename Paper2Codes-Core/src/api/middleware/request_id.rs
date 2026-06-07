use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

/// Middleware to add request ID to requests and responses
pub async fn add_request_id(mut request: Request, next: Next) -> Response {
    // Generate or extract request ID
    let request_id = request
        .headers()
        .get("x-request-id")
        .cloned()
        .unwrap_or_else(|| {
            HeaderValue::from_str(&Uuid::new_v4().to_string())
                .expect("Uuid should be valid header value")
        });

    // Add request ID to request headers if not present
    request.headers_mut().insert(
        axum::http::header::HeaderName::from_static("x-request-id"),
        request_id.clone(),
    );

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers for tracing
    response.headers_mut().insert(
        axum::http::header::HeaderName::from_static("x-request-id"),
        request_id,
    );

    response
}
