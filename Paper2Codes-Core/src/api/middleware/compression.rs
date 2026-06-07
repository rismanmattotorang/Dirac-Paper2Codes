//! Response compression middleware

use axum::{extract::Request, http::header::ACCEPT_ENCODING, middleware::Next, response::Response};

/// Compression middleware using tower-http
/// This is handled by the CompressionLayer in the router setup
/// This file exists for documentation and potential custom compression logic

/// Check if response should be compressed
pub fn should_compress(content_type: Option<&str>) -> bool {
    if let Some(ct) = content_type {
        // Compress text-based content types
        ct.starts_with("text/")
            || ct.starts_with("application/json")
            || ct.starts_with("application/javascript")
            || ct.starts_with("application/xml")
            || ct.starts_with("application/xhtml+xml")
            || ct.starts_with("application/atom+xml")
            || ct.starts_with("application/rss+xml")
            || ct.starts_with("image/svg+xml")
    } else {
        false
    }
}

/// Get preferred encoding from Accept-Encoding header
pub fn get_preferred_encoding(request: &Request) -> Option<&'static str> {
    request
        .headers()
        .get(ACCEPT_ENCODING)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            // Check for brotli first (better compression)
            if s.contains("br") {
                Some("br")
            } else if s.contains("gzip") {
                Some("gzip")
            } else if s.contains("deflate") {
                Some("deflate")
            } else {
                None
            }
        })
}

/// Compression middleware function (if custom logic needed)
pub async fn compression_middleware(request: Request, next: Next) -> Response {
    // In practice, tower-http's CompressionLayer handles this
    // This function is here for potential custom compression logic
    next.run(request).await
}
