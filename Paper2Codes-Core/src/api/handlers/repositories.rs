use crate::api::state::AppState;
use crate::api::types::requests::PaginationQuery;
use crate::api::types::responses::{ApiResponse, PaginatedResponse};
use crate::api::utils::{error_response, get_request_id};
use crate::types::Repository;
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::sync::Arc;
use tracing::{error, info};

/// List repositories with pagination
/// GET /api/repositories
pub async fn list_repositories(
    Extension(state): Extension<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
    headers: HeaderMap,
) -> Result<Json<PaginatedResponse<Repository>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    // Validate pagination parameters
    if let Err(validation_err) = pagination.validate_pagination() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            &validation_err,
            request_id,
        ));
    }

    // Query repositories from storage
    let repos_result = if state.config.storage.enabled {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.list_repositories().await
        } else {
            Ok(vec![])
        }
    } else {
        // Storage not enabled, return empty list
        Ok(vec![])
    };

    match repos_result {
        Ok(repositories) => {
            // Apply pagination manually since list_repositories doesn't support filters yet
            let total = repositories.len();
            let offset = pagination.offset();
            let limit = pagination.per_page;
            let paginated_repos: Vec<Repository> =
                repositories.into_iter().skip(offset).take(limit).collect();

            let response = PaginatedResponse::with_request_id(
                paginated_repos,
                pagination.page,
                pagination.per_page,
                total,
                request_id.clone(),
            );

            info!(request_id = %request_id, count = total, "Listed repositories");
            Ok(Json(response))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to list repositories");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve repositories: {}", e),
                request_id,
            ))
        }
    }
}

/// Get repository by ID
/// GET /api/repositories/:id
pub async fn get_repository(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Repository>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    let id_clone = id.clone();
    let repo_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_repository(&id_clone).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match repo_result {
        Ok(Some(repo)) => {
            info!(request_id = %request_id, repository_id = %id, "Retrieved repository");
            Ok(Json(ApiResponse::with_request_id(repo, request_id)))
        }
        Ok(None) => {
            info!(request_id = %request_id, repository_id = %id, "Repository not found");
            Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Repository with id {} not found", id),
                request_id,
            ))
        }
        Err(e) => {
            error!(request_id = %request_id, repository_id = %id, error = %e, "Failed to get repository");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve repository: {}", e),
                request_id,
            ))
        }
    }
}
