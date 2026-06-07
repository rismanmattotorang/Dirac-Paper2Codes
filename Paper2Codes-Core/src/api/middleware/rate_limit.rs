//! Rate limiting middleware using governor

use crate::api::auth::{Claims, JwtService};
use crate::api::state::AppState;
use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock, middleware::NoOpMiddleware, state::keyed::DefaultKeyedStateStore, Quota,
    RateLimiter,
};
use std::{num::NonZeroU32, sync::Arc};

/// Rate limiter key type
type KeyedRateLimiter =
    RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock, NoOpMiddleware>;

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    /// General API rate limiter (per user)
    general_limiter: Arc<KeyedRateLimiter>,
    /// Auth endpoints rate limiter (per IP)
    auth_limiter: Arc<KeyedRateLimiter>,
    /// Upload endpoints rate limiter (per user)
    upload_limiter: Arc<KeyedRateLimiter>,
    /// LLM endpoints rate limiter (per user)
    llm_limiter: Arc<KeyedRateLimiter>,
}

impl RateLimitMiddleware {
    /// Create a new rate limit middleware
    pub fn new(config: &crate::config::RateLimitConfig) -> Self {
        // General API: 100 requests/minute per user
        let general_quota =
            Quota::per_minute(NonZeroU32::new(config.general_requests_per_minute).unwrap());
        let general_limiter = Arc::new(RateLimiter::keyed(general_quota));

        // Auth endpoints: 5 requests/minute per IP
        let auth_quota =
            Quota::per_minute(NonZeroU32::new(config.auth_requests_per_minute).unwrap());
        let auth_limiter = Arc::new(RateLimiter::keyed(auth_quota));

        // Upload endpoints: 10 requests/minute per user
        let upload_quota =
            Quota::per_minute(NonZeroU32::new(config.upload_requests_per_minute).unwrap());
        let upload_limiter = Arc::new(RateLimiter::keyed(upload_quota));

        // LLM endpoints: 20 requests/minute per user
        let llm_quota = Quota::per_minute(NonZeroU32::new(config.llm_requests_per_minute).unwrap());
        let llm_limiter = Arc::new(RateLimiter::keyed(llm_quota));

        Self {
            general_limiter,
            auth_limiter,
            upload_limiter,
            llm_limiter,
        }
    }

    /// Get rate limiter for a specific endpoint
    fn get_limiter(&self, path: &str) -> &Arc<KeyedRateLimiter> {
        if path.starts_with("/api/auth/") {
            &self.auth_limiter
        } else if path.starts_with("/api/papers/upload") || path.starts_with("/api/files/upload") {
            &self.upload_limiter
        } else if path.starts_with("/api/llm/") || path.contains("/generate") {
            &self.llm_limiter
        } else {
            &self.general_limiter
        }
    }

    /// Get identifier for rate limiting (user ID or IP)
    fn get_identifier(
        &self,
        request: &Request,
        path: &str,
        app_state: Option<&Arc<AppState>>,
    ) -> String {
        // For auth endpoints, use IP address
        if path.starts_with("/api/auth/") {
            self.get_client_ip(request)
        } else {
            // For other endpoints, try to get user ID from JWT token
            // If not available, fall back to IP
            if let Some(app_state) = app_state {
                if let Some(claims) = self.extract_claims_from_request(request, app_state) {
                    return format!("user:{}", claims.sub);
                }
            }

            // Fall back to IP
            self.get_client_ip(request)
        }
    }

    /// Extract client IP address from request headers
    fn get_client_ip(&self, request: &Request) -> String {
        request
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next().map(|s| s.trim()))
            .or_else(|| {
                request
                    .headers()
                    .get("x-real-ip")
                    .and_then(|h| h.to_str().ok())
            })
            .unwrap_or("unknown")
            .to_string()
    }

    /// Extract JWT claims from request (for user identification)
    fn extract_claims_from_request(
        &self,
        request: &Request,
        app_state: &Arc<AppState>,
    ) -> Option<Claims> {
        let auth_header = request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())?;

        let token = auth_header.strip_prefix("Bearer ")?;

        // Try to validate token and extract claims
        let jwt_service = JwtService::new(&app_state.config.api.auth).ok()?;
        jwt_service.validate_access_token(token).ok()
    }
}

/// Rate limiting middleware function
pub async fn rate_limit_middleware(
    axum::extract::Extension(rate_limiter): axum::extract::Extension<Arc<RateLimitMiddleware>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let path = request.uri().path();

    // Get app state for JWT validation (if needed)
    let app_state = request.extensions().get::<Arc<AppState>>().cloned();

    let identifier = rate_limiter.get_identifier(&request, path, app_state.as_ref());

    // Bypass rate limiting for localhost/127.0.0.1 in development
    // This allows easier development and testing
    let is_localhost = identifier.contains("127.0.0.1")
        || identifier.contains("localhost")
        || identifier == "unknown";

    if is_localhost {
        // Skip rate limiting for localhost
        let response = next.run(request).await;
        return Ok(response);
    }

    let limiter = rate_limiter.get_limiter(path);

    // Check rate limit
    match limiter.check_key(&identifier) {
        Ok(_) => {
            // Rate limit OK, continue
            let mut response = next.run(request).await;

            // Add rate limit headers
            // Note: governor doesn't expose remaining quota directly,
            // so we use a placeholder. In production, you might want to
            // track this separately or use a different rate limiting library
            response.headers_mut().insert(
                "x-ratelimit-limit",
                HeaderValue::from_str("1000").unwrap(), // Updated to match new default
            );
            response.headers_mut().insert(
                "x-ratelimit-remaining",
                HeaderValue::from_str("999").unwrap(), // Placeholder
            );
            response.headers_mut().insert(
                "x-ratelimit-reset",
                HeaderValue::from_str(&(chrono::Utc::now().timestamp() + 60).to_string()).unwrap(),
            );

            Ok(response)
        }
        Err(_) => {
            // Rate limit exceeded
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Rate limit exceeded for identifier: {}. Please try again later.",
                    identifier
                ),
            ))
        }
    }
}
