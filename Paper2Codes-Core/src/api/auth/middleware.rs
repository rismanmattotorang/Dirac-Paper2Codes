//! Authentication and authorization middleware

use crate::api::auth::{Claims, JwtService, UserRole};
use crate::api::state::AppState;
use axum::{
    extract::Extension,
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware(
    state: Extension<Arc<AppState>>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, (StatusCode, String)> {
    // Skip auth for public endpoints
    let path = request.uri().path();
    if is_public_endpoint(path) {
        return Ok(next.run(request).await);
    }

    // Get app state
    let app_state = state.0;

    // Check if authentication is disabled (for development)
    let auth_disabled = std::env::var("PAPER2CODES_DISABLE_AUTH")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // If auth is disabled, allow request to proceed without authentication
    if auth_disabled {
        // Authentication disabled; proceed without claims
        return Ok(next.run(request).await);
    }

    // Require authentication
    let auth_header = auth_header.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        )
    })?;

    // Parse Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header format. Expected 'Bearer <token>'".to_string(),
        )
    })?;

    // Validate token
    let jwt_service = JwtService::new(&app_state.config.api.auth).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create JWT service: {}", e),
        )
    })?;

    let claims = jwt_service.validate_access_token(token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token".to_string(),
        )
    })?;

    // Attach claims to request extensions for use in handlers
    let mut request = request;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Require authentication - returns error if not authenticated
pub async fn require_auth(
    state: Extension<Arc<AppState>>,
    request: Request,
    next: Next,
) -> std::result::Result<Response, (StatusCode, String)> {
    auth_middleware(state, request, next).await
}

/// Require specific role - returns error if user doesn't have required role
pub async fn require_role(
    required_role: UserRole,
    request: Request,
    next: Next,
) -> std::result::Result<Response, (StatusCode, String)> {
    // First check authentication
    let mut request = request;

    // Get app state
    let app_state = request.extensions().get::<Arc<AppState>>().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "AppState not found".to_string(),
        )
    })?;

    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            )
        })?;

    // Parse Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header format".to_string(),
        )
    })?;

    // Validate token
    let jwt_service = JwtService::new(&app_state.config.api.auth).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create JWT service: {}", e),
        )
    })?;

    let claims = jwt_service.validate_access_token(token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token".to_string(),
        )
    })?;

    // Check if user has required role
    let user_roles: Vec<UserRole> = claims
        .roles
        .iter()
        .filter_map(|r| UserRole::from_str(r))
        .collect();

    let has_role = user_roles
        .iter()
        .any(|role| role.can_access(&required_role));

    if !has_role {
        return Err((
            StatusCode::FORBIDDEN,
            format!(
                "Insufficient permissions. Required role: {}",
                required_role.as_str()
            ),
        ));
    }

    // Attach claims to request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Check if endpoint is public (doesn't require authentication)
fn is_public_endpoint(path: &str) -> bool {
    path == "/api/health"
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/register")
        || path == "/api/docs"
        || path == "/api/openapi.json"
}

/// Helper function to extract claims from request
pub fn extract_claims(request: &Request) -> Option<Claims> {
    request.extensions().get::<Claims>().cloned()
}

/// Helper function to check if user has role
pub fn has_role(claims: &Claims, role: &str) -> bool {
    claims.has_role(role)
}

/// Helper function to check if user has any of the roles
pub fn has_any_role(claims: &Claims, roles: &[&str]) -> bool {
    claims.has_any_role(roles)
}
