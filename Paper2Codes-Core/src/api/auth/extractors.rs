//! Axum extractors for authentication

use crate::api::auth::{Claims, JwtService};
use crate::api::state::AppState;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use std::sync::Arc;

/// Extract authenticated user from request
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}

impl From<Claims> for CurrentUser {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            email: claims.email,
            roles: claims.roles,
        }
    }
}

/// Extract authenticated user with full claims
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Get JWT service from app state
        let app_state = parts.extensions.get::<Arc<AppState>>().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AppState not found".to_string(),
            )
        })?;

        // Extract token from Authorization header
        let auth_header = parts
            .headers
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

        Ok(CurrentUser::from(claims))
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        // Get JWT service from app state
        let app_state = parts.extensions.get::<Arc<AppState>>().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AppState not found".to_string(),
            )
        })?;

        // Extract token from Authorization header
        let auth_header = parts
            .headers
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

        Ok(AuthenticatedUser { claims })
    }
}
