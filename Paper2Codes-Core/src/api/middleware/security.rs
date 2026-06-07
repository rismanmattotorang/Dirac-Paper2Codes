//! Security headers middleware

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};
use tower_http::set_header::SetResponseHeaderLayer;

/// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    // Set security headers
    let headers = response.headers_mut();

    // X-Content-Type-Options: nosniff
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // X-Frame-Options: DENY
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    // X-XSS-Protection: 1; mode=block
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    // Strict-Transport-Security (HSTS) - only in production with HTTPS
    // Uncomment in production:
    // headers.insert(
    //     header::STRICT_TRANSPORT_SECURITY,
    //     HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    // );

    // Content-Security-Policy
    // Adjust based on your needs
    let csp = "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'";
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_str(csp)
            .unwrap_or_else(|_| HeaderValue::from_static("default-src 'self'")),
    );

    // Referrer-Policy
    headers.insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions-Policy (formerly Feature-Policy)
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    response
}

/// Create security headers layer
pub fn security_headers_layer() -> SetResponseHeaderLayer<HeaderValue> {
    // This is a simplified version - in practice, you'd use tower-http layers
    // For now, we use the middleware function above
    SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    )
}
