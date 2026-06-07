use crate::api::state::AppState;
use crate::api::types::requests::{CreatePaperRequest, PaginationQuery, UpdatePaperRequest};
use crate::api::types::responses::{ApiResponse, PaginatedResponse};
use crate::api::utils::{error_response, get_request_id, validation_error_response};
use crate::storage::filters::PaperFilters;
use crate::types::Paper;
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;
#[cfg(feature = "api")]
use validator::Validate;

/// List papers with pagination
/// GET /api/papers
pub async fn list_papers(
    Extension(state): Extension<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
    headers: HeaderMap,
) -> Result<Json<PaginatedResponse<Paper>>, (StatusCode, Json<serde_json::Value>)> {
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

    // Convert pagination to filters
    let filters = PaperFilters {
        limit: Some(pagination.per_page),
        offset: Some(pagination.offset()),
        ..Default::default()
    };

    // Query papers from storage
    let papers_result = if state.config.storage.enabled {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            let filters_clone = filters.clone();
            storage_manager.list_papers(filters_clone).await
        } else {
            Ok(vec![])
        }
    } else {
        // Storage not enabled, return empty list
        Ok(vec![])
    };

    match papers_result {
        Ok(papers) => {
            // Note: The total count is an approximation based on returned results
            // For accurate pagination, the storage layer should support count queries
            // For now, if we got fewer results than requested, we know we're at the end
            let total = if papers.len() < pagination.per_page {
                // We're on the last page, calculate total from current page
                pagination.offset() + papers.len()
            } else {
                // We might have more, estimate total (will be updated when storage supports count)
                pagination.offset() + papers.len() + 1
            };

            let response = PaginatedResponse::with_request_id(
                papers.clone(),
                pagination.page,
                pagination.per_page,
                total,
                request_id.clone(),
            );

            info!(request_id = %request_id, count = papers.len(), total = total, "Listed papers");
            Ok(Json(response))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to list papers");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve papers: {}", e),
                request_id,
            ))
        }
    }
}

/// Get paper by ID
/// GET /api/papers/:id
pub async fn get_paper(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Paper>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    let paper_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_paper(&id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match paper_result {
        Ok(Some(paper)) => {
            info!(request_id = %request_id, paper_id = %id, "Retrieved paper");
            Ok(Json(ApiResponse::with_request_id(paper, request_id)))
        }
        Ok(None) => {
            info!(request_id = %request_id, paper_id = %id, "Paper not found");
            Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Paper with id {} not found", id),
                request_id,
            ))
        }
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to get paper");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve paper: {}", e),
                request_id,
            ))
        }
    }
}

/// Create a new paper
/// POST /api/papers
pub async fn create_paper(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreatePaperRequest>,
) -> Result<Json<ApiResponse<Paper>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    // Validate request using validator crate
    #[cfg(feature = "api")]
    if let Err(validation_err) = request.validate() {
        return Err(validation_error_response(&validation_err, request_id));
    }

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Create paper from request
    let paper = Paper {
        id: Uuid::new_v4().to_string(),
        title: request.title,
        abstract_text: request.abstract_text,
        segments: vec![],
        algorithms: vec![],
        equations: vec![],
        figures: vec![],
        tables: vec![],
        references: vec![],
        metadata: crate::types::PaperMetadata {
            authors: vec![],
            year: None,
            venue: None,
            keywords: vec![],
            file_path: None,
        },
    };

    // Save to storage
    let paper_id = paper.id.clone();
    let paper_clone = paper.clone();
    let save_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.save_paper(&paper_clone).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match save_result {
        Ok(_) => {
            info!(request_id = %request_id, paper_id = %paper_id, "Created paper");
            Ok(Json(ApiResponse::with_request_id(paper, request_id)))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to create paper");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to create paper: {}", e),
                request_id,
            ))
        }
    }
}

/// Update a paper
/// PUT /api/papers/:id
pub async fn update_paper(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpdatePaperRequest>,
) -> Result<Json<ApiResponse<Paper>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    // Validate request using validator crate
    #[cfg(feature = "api")]
    if let Err(validation_err) = request.validate() {
        return Err(validation_error_response(&validation_err, request_id));
    }

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Get existing paper
    let paper_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_paper(&id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match paper_result {
        Ok(Some(mut paper)) => {
            // Update fields
            if let Some(title) = request.title {
                if !title.trim().is_empty() {
                    paper.title = title;
                }
            }
            if let Some(abstract_text) = request.abstract_text {
                paper.abstract_text = abstract_text;
            }

            // Save updated paper
            let paper_clone = paper.clone();
            let save_result = {
                let storage_guard = state.storage.read().await;
                if let Some(storage_manager) = storage_guard.as_ref() {
                    storage_manager.save_paper(&paper_clone).await
                } else {
                    Err(crate::error::Paper2CodesError::Storage(
                        crate::storage::errors::StorageError::NotConnected,
                    ))
                }
            };

            match save_result {
                Ok(_) => {
                    info!(request_id = %request_id, paper_id = %id, "Updated paper");
                    Ok(Json(ApiResponse::with_request_id(paper, request_id)))
                }
                Err(e) => {
                    error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to update paper");
                    Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "STORAGE_ERROR",
                        &format!("Failed to update paper: {}", e),
                        request_id,
                    ))
                }
            }
        }
        Ok(None) => Err(error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Paper with id {} not found", id),
            request_id,
        )),
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to get paper for update");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve paper: {}", e),
                request_id,
            ))
        }
    }
}

/// Delete a paper
/// DELETE /api/papers/:id
pub async fn delete_paper(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Check if paper exists first
    let paper_exists = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_paper(&id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match paper_exists {
        Ok(Some(_)) => {
            // Delete paper
            let id_clone = id.clone();
            let delete_result = {
                let storage_guard = state.storage.read().await;
                if let Some(storage_manager) = storage_guard.as_ref() {
                    storage_manager.delete_paper(&id_clone).await
                } else {
                    Err(crate::error::Paper2CodesError::Storage(
                        crate::storage::errors::StorageError::NotConnected,
                    ))
                }
            };

            match delete_result {
                Ok(_) => {
                    info!(request_id = %request_id, paper_id = %id, "Deleted paper");
                    Ok(StatusCode::NO_CONTENT)
                }
                Err(e) => {
                    error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to delete paper");
                    Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "STORAGE_ERROR",
                        &format!("Failed to delete paper: {}", e),
                        request_id,
                    ))
                }
            }
        }
        Ok(None) => Err(error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Paper with id {} not found", id),
            request_id,
        )),
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to check paper existence");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to check paper: {}", e),
                request_id,
            ))
        }
    }
}
