//! Cache middleware for API responses
//!
//! Provides HTTP response caching with ETag and Cache-Control headers

use crate::api::cache::CacheKey;
use crate::api::state::AppState;
use axum::{
    extract::Request,
    http::{
        header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH},
        HeaderValue, StatusCode,
    },
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tracing::{trace, warn};

/// Cache middleware that caches GET responses
pub async fn cache_middleware(request: Request, next: Next) -> Response {
    // Only cache GET requests
    if request.method() != axum::http::Method::GET {
        return next.run(request).await;
    }

    // Extract state from request extensions (axum stores state here when using with_state)
    let state = match request.extensions().get::<Arc<AppState>>() {
        Some(state) => state.clone(),
        None => {
            // State not available, skip caching
            return next.run(request).await;
        }
    };

    // Check if cache is enabled
    #[cfg(feature = "api")]
    {
        if !state.config.api.cache.enabled {
            return next.run(request).await;
        }
    }

    #[cfg(not(feature = "api"))]
    {
        // Cache not available without api feature
        return next.run(request).await;
    }

    #[cfg(feature = "api")]
    let cache = state.cache.clone();

    // Generate cache key from request URI
    let cache_key = generate_cache_key(&request);

    // Check If-None-Match header for conditional requests
    let if_none_match = request.headers().get(IF_NONE_MATCH);

    #[cfg(feature = "api")]
    {
        // Try to get from cache
        if let Some(cached_data) = cache.get(&cache_key).await {
            // Generate ETag from cached data
            let etag = generate_etag(&cached_data);

            // Check ETag match for conditional requests
            if let Some(if_none_match_value) = if_none_match {
                if if_none_match_value == &etag {
                    // Return 304 Not Modified
                    let mut response = Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .body(axum::body::Body::empty())
                        .unwrap();

                    // Add cache headers
                    response
                        .headers_mut()
                        .insert(ETAG, HeaderValue::from_str(&etag).unwrap());
                    response.headers_mut().insert(
                        CACHE_CONTROL,
                        HeaderValue::from_static("public, max-age=300"),
                    );

                    trace!(key = %cache_key, "Cache hit: 304 Not Modified");
                    return response;
                }
            }

            // Return cached response
            trace!(key = %cache_key, "Cache hit");
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .body(axum::body::Body::from(cached_data))
                .unwrap();

            // Add cache headers
            response
                .headers_mut()
                .insert(ETAG, HeaderValue::from_str(&etag).unwrap());
            response.headers_mut().insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=300"),
            );

            return response;
        }

        // Cache miss - proceed with request
        trace!(key = %cache_key, "Cache miss");
        let response = next.run(request).await;

        // Cache successful responses (2xx status codes)
        if response.status().is_success() {
            // Clone response for caching
            let (parts, body) = response.into_parts();
            let body_bytes = axum::body::to_bytes(body, usize::MAX).await;

            match body_bytes {
                Ok(bytes) => {
                    // Generate ETag from response body
                    let etag = generate_etag(&bytes);

                    // Store in cache with TTL
                    let content_type = parts
                        .headers
                        .get(axum::http::header::CONTENT_TYPE)
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("application/json")
                        .to_string();

                    // Store raw response body in cache
                    cache
                        .set(
                            &cache_key,
                            bytes.to_vec(),
                            content_type,
                            Some(Duration::from_secs(300)), // 5 minutes default
                        )
                        .await;

                    // Rebuild response with cache headers
                    let mut response = Response::from_parts(parts, axum::body::Body::from(bytes));
                    response
                        .headers_mut()
                        .insert(ETAG, HeaderValue::from_str(&etag).unwrap());
                    response.headers_mut().insert(
                        CACHE_CONTROL,
                        HeaderValue::from_static("public, max-age=300"),
                    );

                    response
                }
                Err(e) => {
                    // Failed to read body, rebuild response without caching
                    warn!("Failed to read response body for caching: {}", e);
                    Response::from_parts(parts, axum::body::Body::empty())
                }
            }
        } else {
            response
        }
    }

    // Fallback when api feature is not enabled
    #[cfg(not(feature = "api"))]
    {
        next.run(request).await
    }
}

/// Generate cache key from request
fn generate_cache_key(request: &Request) -> CacheKey {
    let uri = request.uri();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    // Generate key based on path
    if path.starts_with("/api/papers/") {
        if let Some(id) = path.strip_prefix("/api/papers/") {
            if !id.contains('/') {
                return CacheKey::paper(id);
            }
        }
    } else if path.starts_with("/api/repositories/") {
        if let Some(id) = path.strip_prefix("/api/repositories/") {
            if !id.contains('/') {
                return CacheKey::repository(id);
            }
        }
    } else if path.starts_with("/api/tasks/") {
        if let Some(id) = path.strip_prefix("/api/tasks/") {
            if !id.contains('/') {
                return CacheKey::task(id);
            }
        }
    } else if path == "/api/papers" {
        return CacheKey::list("papers", query);
    } else if path == "/api/repositories" {
        return CacheKey::list("repositories", query);
    } else if path == "/api/tasks" {
        return CacheKey::list("tasks", query);
    }

    // Default: use full URI as key
    let full_uri = format!("{}?{}", path, query);
    CacheKey::search(&full_uri)
}

/// Generate ETag from response body
fn generate_etag(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let hash = hasher.finalize();
    format!("\"{:x}\"", hash)
}
