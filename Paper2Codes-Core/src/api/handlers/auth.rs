//! Authentication handlers

use crate::api::auth::{CurrentUser, User, UserRole, UserService};
use crate::api::state::AppState;
use crate::api::types::errors::ApiError as ApiErrorResponse;
use crate::api::types::responses::ApiResponse;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
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

    // Load the user from the store (by username or email) and verify password.
    // A generic error avoids leaking which accounts exist.
    let invalid = || {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::unauthorized("Invalid credentials")),
        )
    };
    let user = app_state
        .user_store
        .find_by_login(&request.email)
        .await
        .ok_or_else(invalid)?;

    if !user.verify_password(&request.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorResponse::internal_error(&format!(
                "Password verification failed: {}",
                e
            ))),
        )
    })? {
        return Err(invalid());
    }

    issue_session(&app_state, &user).await
}

/// Issue access + refresh tokens for `user` and persist a session.
async fn issue_session(
    app_state: &Arc<AppState>,
    user: &User,
) -> std::result::Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiErrorResponse>)> {
    let access_token = app_state
        .jwt_service
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

    let refresh_token = app_state
        .jwt_service
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
    let refresh_expiry =
        chrono::Utc::now() + chrono::Duration::seconds(app_state.config.api.auth.refresh_expiration as i64);
    app_state
        .session_store
        .create(user.id.clone(), refresh_token.clone(), refresh_expiry)
        .await;

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in,
        user: UserInfo::from(user.clone()),
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

    // Create the user in the store (rejects duplicate username/email). The
    // first registered account becomes an admin; subsequent users are regular.
    let first_user = app_state.user_store.count().await == 0;
    let roles = if first_user {
        vec![UserRole::Admin]
    } else {
        vec![UserRole::User]
    };

    let user = app_state
        .user_store
        .register(request.email, request.username, &request.password, roles)
        .await
        .map_err(|e| {
            (
                StatusCode::CONFLICT,
                Json(ApiErrorResponse::validation_error_simple(&e.to_string())),
            )
        })?;

    issue_session(&app_state, &user).await
}

/// Refresh token handler
pub async fn refresh_token(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> std::result::Result<Json<ApiResponse<LoginResponse>>, (StatusCode, Json<ApiErrorResponse>)> {
    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorResponse::unauthorized(
                "Invalid or expired refresh token",
            )),
        )
    };

    // Validate the refresh token signature/expiry...
    let claims = app_state
        .jwt_service
        .validate_refresh_token(&request.refresh_token)
        .map_err(|_| unauthorized())?;

    // ...and confirm the session still exists (i.e. has not been revoked).
    if app_state
        .session_store
        .get_by_refresh(&request.refresh_token)
        .await
        .is_none()
    {
        return Err(unauthorized());
    }

    // Rotate: revoke the old session, then load the user and issue a new one.
    app_state
        .session_store
        .revoke_by_refresh(&request.refresh_token)
        .await;

    let user = app_state
        .user_store
        .find_by_id(&claims.sub)
        .await
        .ok_or_else(unauthorized)?;

    issue_session(&app_state, &user).await
}

/// Get current user handler
pub async fn get_current_user(
    Extension(app_state): Extension<Arc<AppState>>,
    user: CurrentUser,
) -> Json<ApiResponse<UserInfo>> {
    let username = app_state
        .user_store
        .find_by_id(&user.user_id)
        .await
        .map(|u| u.username)
        .unwrap_or_else(|| "user".to_string());
    Json(ApiResponse::success(UserInfo {
        id: user.user_id,
        email: user.email,
        username,
        roles: user.roles,
    }))
}

/// Logout handler — revokes the server-side session for the given refresh token.
pub async fn logout(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(request): Json<RefreshTokenRequest>,
) -> Json<ApiResponse<()>> {
    app_state
        .session_store
        .revoke_by_refresh(&request.refresh_token)
        .await;
    Json(ApiResponse::success(()))
}

// ---- Personal API tokens ----

/// Request to mint a new personal API token.
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    /// Optional lifetime in days; omit for a non-expiring token.
    pub expires_in_days: Option<u32>,
}

/// Public view of a token (never includes the secret).
#[derive(Debug, Serialize)]
pub struct ApiTokenInfo {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<crate::api::auth::ApiToken> for ApiTokenInfo {
    fn from(t: crate::api::auth::ApiToken) -> Self {
        Self {
            id: t.id,
            name: t.name,
            prefix: t.prefix,
            created_at: t.created_at,
            last_used_at: t.last_used_at,
            expires_at: t.expires_at,
        }
    }
}

/// Response returned exactly once on creation — includes the plaintext token.
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    /// The plaintext token. Shown only here; it cannot be retrieved again.
    pub token: String,
    #[serde(flatten)]
    pub info: ApiTokenInfo,
}

/// Create a personal API token for the authenticated user.
pub async fn create_api_token(
    Extension(app_state): Extension<Arc<AppState>>,
    user: CurrentUser,
    Json(request): Json<CreateTokenRequest>,
) -> std::result::Result<Json<ApiResponse<CreateTokenResponse>>, (StatusCode, Json<ApiErrorResponse>)>
{
    let name = request.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::validation_error_simple(
                "Token name must be between 1 and 100 characters",
            )),
        ));
    }
    let expires_at = request
        .expires_in_days
        .map(|d| chrono::Utc::now() + chrono::Duration::days(d as i64));

    let (token, plaintext) = app_state
        .api_token_store
        .create(user.user_id, name.to_string(), expires_at)
        .await;

    Ok(Json(ApiResponse::success(CreateTokenResponse {
        token: plaintext,
        info: ApiTokenInfo::from(token),
    })))
}

/// List the authenticated user's API tokens (without secrets).
pub async fn list_api_tokens(
    Extension(app_state): Extension<Arc<AppState>>,
    user: CurrentUser,
) -> Json<ApiResponse<Vec<ApiTokenInfo>>> {
    let mut tokens: Vec<ApiTokenInfo> = app_state
        .api_token_store
        .list_for_user(&user.user_id)
        .await
        .into_iter()
        .map(ApiTokenInfo::from)
        .collect();
    tokens.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(ApiResponse::success(tokens))
}

/// Revoke one of the authenticated user's API tokens.
pub async fn revoke_api_token(
    Extension(app_state): Extension<Arc<AppState>>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> std::result::Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiErrorResponse>)> {
    if app_state.api_token_store.revoke(&id, &user.user_id).await {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ApiErrorResponse::new("NOT_FOUND", "Token not found")),
        ))
    }
}

/// Create auth router
pub fn auth_router() -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/register", post(register))
        .route("/api/auth/refresh", post(refresh_token))
        .route("/api/auth/me", get(get_current_user))
        .route("/api/auth/logout", post(logout))
        .route(
            "/api/auth/tokens",
            post(create_api_token).get(list_api_tokens),
        )
        .route("/api/auth/tokens/:id", delete(revoke_api_token))
}
