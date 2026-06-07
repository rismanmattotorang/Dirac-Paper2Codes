//! Authentication handlers

use crate::api::auth::{CurrentUser, JwtService, User, UserRole, UserService};
use crate::api::state::AppState;
use crate::api::types::errors::ApiError as ApiErrorResponse;
use crate::api::types::responses::ApiResponse;
use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(feature = "api")]
use validator::Validate;

/// Login request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api", derive(validator::Validate))]
pub struct LoginRequest {
    #[cfg_attr(feature = "api", validate(email(message = "Invalid email format")))]
    pub email: String,
    #[cfg_attr(
        feature = "api",
        validate(length(min = 8, message = "Password must be at least 8 characters"))
    )]
    pub password: String,
}

/// Register request
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "api", derive(validator::Validate))]
pub struct RegisterRequest {
    #[cfg_attr(feature = "api", validate(email(message = "Invalid email format")))]
    pub email: String,
    #[cfg_attr(
        feature = "api",
        validate(length(
            min = 3,
            max = 30,
            message = "Username must be between 3 and 30 characters"
        ))
    )]
    pub username: String,
    #[cfg_attr(
        feature = "api",
        validate(length(min = 8, message = "Password must be at least 8 characters"))
    )]
    pub password: String,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
}

/// User info (without sensitive data)
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub username: String,
    pub roles: Vec<String>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        let roles = user.roles_as_strings();
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            roles,
        }
    }
}

/// Login handler
pub async fn login(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> std::result::Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiErrorResponse>)> {
    // Validate request
    #[cfg(feature = "api")]
    {
        if let Err(errors) = request.validate() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse::validation_error(
                    "Validation failed",
                    errors,
                )),
            ));
        }
    }

    // TODO: Load user from database
    // For now, this is a placeholder implementation
    // In production, you would:
    // 1. Query database for user by email
    // 2. Verify password
    // 3. Generate tokens

    // Placeholder: Create a mock user for demonstration
    // In real implementation, load from database
    let user = User::new(
        request.email.clone(),
        "demo_user".to_string(),
        &request.password,
        vec![UserRole::User],
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Failed to create user: {}",
                e
            ))),
        )
    })?;

    // Verify password
    if !user.verify_password(&request.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Password verification failed: {}",
                e
            ))),
        )
    })? {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::unauthorized("Invalid email or password")),
        ));
    }

    // Generate tokens
    let jwt_service = JwtService::new(&app_state.config.api.auth).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Failed to create JWT service: {}",
                e
            ))),
        )
    })?;

    let access_token = jwt_service
        .generate_access_token(user.id.clone(), user.email.clone(), user.roles_as_strings())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate access token: {}",
                    e
                ))),
            )
        })?;

    let refresh_token = jwt_service
        .generate_refresh_token(user.id.clone(), user.email.clone(), user.roles_as_strings())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate refresh token: {}",
                    e
                ))),
            )
        })?;

    let expires_in = app_state.config.api.auth.jwt_expiration;

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserInfo::from(user),
    })))
}

/// Register handler
pub async fn register(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> std::result::Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiErrorResponse>)> {
    // Validate request
    #[cfg(feature = "api")]
    {
        if let Err(errors) = request.validate() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse::validation_error(
                    "Validation failed",
                    errors,
                )),
            ));
        }
    }

    // Validate email format
    if !UserService::validate_email(&request.email) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::validation_error_simple(
                "Invalid email format",
            )),
        ));
    }

    // Validate username format
    if !UserService::validate_username(&request.username) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::validation_error_simple(
                "Invalid username format",
            )),
        ));
    }

    // Validate password strength
    let min_length = app_state.config.api.auth.password_min_length;
    if !UserService::validate_password(&request.password, min_length) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::validation_error_simple(
                &format!("Password must be at least {} characters and contain uppercase, lowercase, and digits", min_length)
            )),
        ));
    }

    // TODO: Check if user already exists in database
    // TODO: Create user in database

    // Create user
    let user = User::new(
        request.email,
        request.username,
        &request.password,
        vec![UserRole::User], // Default role
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Failed to create user: {}",
                e
            ))),
        )
    })?;

    // Generate tokens
    let jwt_service = JwtService::new(&app_state.config.api.auth).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Failed to create JWT service: {}",
                e
            ))),
        )
    })?;

    let access_token = jwt_service
        .generate_access_token(user.id.clone(), user.email.clone(), user.roles_as_strings())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate access token: {}",
                    e
                ))),
            )
        })?;

    let refresh_token = jwt_service
        .generate_refresh_token(user.id.clone(), user.email.clone(), user.roles_as_strings())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate refresh token: {}",
                    e
                ))),
            )
        })?;

    let expires_in = app_state.config.api.auth.jwt_expiration;

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserInfo::from(user),
    })))
}

/// Refresh token handler
pub async fn refresh_token(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> std::result::Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiErrorResponse>)> {
    let jwt_service = JwtService::new(&app_state.config.api.auth).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Failed to create JWT service: {}",
                e
            ))),
        )
    })?;

    // Validate refresh token
    let claims = jwt_service
        .validate_refresh_token(&request.refresh_token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse::unauthorized(
                    "Invalid or expired refresh token",
                )),
            )
        })?;

    // TODO: Load user from database using claims.sub
    // For now, use claims directly

    // Generate new tokens
    let access_token = jwt_service
        .generate_access_token(
            claims.sub.clone(),
            claims.email.clone(),
            claims.roles.clone(),
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate access token: {}",
                    e
                ))),
            )
        })?;

    let refresh_token = jwt_service
        .generate_refresh_token(
            claims.sub.clone(),
            claims.email.clone(),
            claims.roles.clone(),
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::internal_error(&format!(
                    "Failed to generate refresh token: {}",
                    e
                ))),
            )
        })?;

    let expires_in = app_state.config.api.auth.jwt_expiration;

    // TODO: Load user from database for UserInfo
    // For now, create minimal user info from claims
    Ok(Json(ApiResponse::success(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserInfo {
            id: claims.sub,
            email: claims.email,
            username: "user".to_string(), // TODO: Load from database
            roles: claims.roles,
        },
    })))
}

/// Get current user handler
pub async fn get_current_user(user: CurrentUser) -> Json<ApiResponse<UserInfo>> {
    Json(ApiResponse::success(UserInfo {
        id: user.user_id,
        email: user.email,
        username: "user".to_string(), // TODO: Load from database
        roles: user.roles,
    }))
}

/// Logout handler (client-side token removal, server-side session invalidation)
pub async fn logout() -> Json<ApiResponse<()>> {
    // In a stateless JWT system, logout is primarily client-side
    // Server can maintain a blacklist of tokens if needed
    // For now, just return success
    Json(ApiResponse::success(()))
}

/// Create auth router
pub fn auth_router() -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/me", get(get_current_user))
        .route("/api/auth/logout", post(logout))
}
