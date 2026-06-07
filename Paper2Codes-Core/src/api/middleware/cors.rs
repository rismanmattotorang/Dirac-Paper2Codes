use crate::config::Config;
use axum::http::{HeaderName, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Create CORS layer with configuration from Config
pub fn cors_layer(config: &Config) -> CorsLayer {
    let allowed_origins: Vec<axum::http::HeaderValue> = config
        .api
        .cors_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    // Define specific allowed headers (required when using allow_credentials)
    let allowed_headers = vec![
        HeaderName::from_static("content-type"),
        HeaderName::from_static("authorization"),
        HeaderName::from_static("accept"),
        HeaderName::from_static("origin"),
        HeaderName::from_static("x-requested-with"),
        HeaderName::from_static("x-request-id"),
    ];

    // Define specific allowed methods (required when using allow_credentials)
    let allowed_methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
    ];

    if allowed_origins.is_empty() {
        // Default: allow all origins in development (without credentials)
        CorsLayer::permissive()
    } else if allowed_origins.len() == 1 {
        // Single origin - use exact match
        CorsLayer::new()
            .allow_origin(AllowOrigin::exact(allowed_origins[0].clone()))
            .allow_methods(allowed_methods.clone())
            .allow_headers(allowed_headers.clone())
            .allow_credentials(true)
            .expose_headers(vec![HeaderName::from_static("x-request-id")])
    } else {
        // Multiple origins - use predicate to check each origin
        let origins_check = allowed_origins.clone();
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(move |origin, _| {
                origins_check
                    .iter()
                    .any(|allowed| allowed.as_bytes() == origin.as_bytes())
            }))
            .allow_methods(allowed_methods)
            .allow_headers(allowed_headers)
            .allow_credentials(true)
            .expose_headers(vec![HeaderName::from_static("x-request-id")])
    }
}
