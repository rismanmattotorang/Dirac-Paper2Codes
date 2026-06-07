use crate::api::state::AppState;
use crate::api::types::requests::PaginationQuery;
use crate::api::types::responses::{ApiResponse, PaginatedResponse};
use crate::api::utils::{error_response, get_request_id, validation_error_response};
use crate::api::websocket::broadcast;
use crate::types::{Task, TaskContext, TaskStatus, TaskType};
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;
#[cfg(feature = "api")]
use validator::Validate;

/// Create task request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct CreateTaskRequest {
    #[cfg_attr(feature = "api", validate(length(min = 1, max = 500)))]
    pub description: String,
    pub task_type: TaskType,
    pub paper_id: Option<String>,
    pub module_id: Option<String>,
}

/// Update task status request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct UpdateTaskStatusRequest {
    pub status: TaskStatus,
}

/// List tasks with pagination
/// GET /api/tasks
pub async fn list_tasks(
    Extension(state): Extension<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
    headers: HeaderMap,
) -> Result<Json<PaginatedResponse<Task>>, (StatusCode, Json<serde_json::Value>)> {
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

    // Query tasks from storage
    // Get all tasks and apply pagination in-memory
    // Note: For better performance, storage layer should support pagination natively
    let tasks_result = if state.config.storage.enabled {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            // Get all tasks (we'll filter and paginate in-memory for now)
            // In production, this should be done at the storage layer
            storage_manager
                .get_tasks_by_status(TaskStatus::Pending)
                .await
        } else {
            Ok(vec![])
        }
    } else {
        // Storage not enabled, return empty list
        Ok(vec![])
    };

    match tasks_result {
        Ok(mut tasks) => {
            // Sort by created_at descending (newest first) before pagination
            tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            let total = tasks.len();
            let offset = pagination.offset();
            let limit = pagination.per_page;

            // Apply pagination
            let paginated_tasks: Vec<Task> = tasks.into_iter().skip(offset).take(limit).collect();

            // Calculate accurate total for pagination
            let total_count = total;
            let paginated_count = paginated_tasks.len();

            let response = PaginatedResponse::with_request_id(
                paginated_tasks,
                pagination.page,
                pagination.per_page,
                total_count,
                request_id.clone(),
            );

            info!(request_id = %request_id, count = paginated_count, total = total_count, "Listed tasks");
            Ok(Json(response))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to list tasks");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve tasks: {}", e),
                request_id,
            ))
        }
    }
}

/// Get task by ID
/// GET /api/tasks/:id
pub async fn get_task(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Parse UUID from string
    let task_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(e) => {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_UUID",
                &format!("Invalid task ID format: {}", e),
                request_id,
            ));
        }
    };

    let task_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_task(&task_id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match task_result {
        Ok(Some(task)) => {
            info!(request_id = %request_id, task_id = %id, "Retrieved task");
            Ok(Json(ApiResponse::with_request_id(task, request_id)))
        }
        Ok(None) => {
            info!(request_id = %request_id, task_id = %id, "Task not found");
            Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Task with id {} not found", id),
                request_id,
            ))
        }
        Err(e) => {
            error!(request_id = %request_id, task_id = %id, error = %e, "Failed to get task");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve task: {}", e),
                request_id,
            ))
        }
    }
}

/// Create a new task
/// POST /api/tasks
pub async fn create_task(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<serde_json::Value>)> {
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

    // Create task
    let task = Task {
        id: Uuid::new_v4(),
        task_type: request.task_type,
        description: request.description,
        context: TaskContext {
            paper_id: request.paper_id,
            module_id: request.module_id,
            repository_id: None,
            dependencies: vec![],
            metadata: serde_json::Value::Null,
            paper_segments: vec![],
            external_refs: vec![],
            code_context: None,
            specifications: vec![],
        },
        dependencies: vec![],
        status: TaskStatus::Pending,
        agent_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Save to storage
    let task_id = task.id;
    let task_clone = task.clone();
    let save_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.save_task(&task_clone).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match save_result {
        Ok(_) => {
            info!(request_id = %request_id, task_id = %task_id, "Created task");

            // Broadcast task creation via WebSocket
            #[cfg(feature = "api")]
            {
                let _ = broadcast::broadcast_task_update(
                    &state,
                    &task_id.to_string(),
                    "created",
                    0.0,
                    Some("Task created successfully".to_string()),
                    None,
                )
                .await;
            }

            Ok(Json(ApiResponse::with_request_id(task, request_id)))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to create task");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to create task: {}", e),
                request_id,
            ))
        }
    }
}

/// Update task status
/// PUT /api/tasks/:id/status
pub async fn update_task_status(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<UpdateTaskStatusRequest>,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<serde_json::Value>)> {
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

    // Parse UUID from string
    let task_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(e) => {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_UUID",
                &format!("Invalid task ID format: {}", e),
                request_id,
            ));
        }
    };

    // Get existing task
    let task_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_task(&task_id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match task_result {
        Ok(Some(mut task)) => {
            // Update status and timestamp
            task.status = request.status;
            task.updated_at = Utc::now();

            // Save updated task
            let task_clone = task.clone();
            let save_result = {
                let storage_guard = state.storage.read().await;
                if let Some(storage_manager) = storage_guard.as_ref() {
                    storage_manager.save_task(&task_clone).await
                } else {
                    Err(crate::error::Paper2CodesError::Storage(
                        crate::storage::errors::StorageError::NotConnected,
                    ))
                }
            };

            match save_result {
                Ok(_) => {
                    info!(request_id = %request_id, task_id = %id, status = ?task.status, "Updated task status");

                    // Broadcast task update via WebSocket
                    #[cfg(feature = "api")]
                    {
                        let _ = broadcast::broadcast_task_update(
                            &state,
                            &id,
                            &format!("{:?}", task.status),
                            0.5,
                            Some(format!("Task status updated to {:?}", task.status)),
                            None,
                        )
                        .await;
                    }

                    Ok(Json(ApiResponse::with_request_id(task, request_id)))
                }
                Err(e) => {
                    error!(request_id = %request_id, task_id = %id, error = %e, "Failed to update task status");
                    Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "STORAGE_ERROR",
                        &format!("Failed to update task: {}", e),
                        request_id,
                    ))
                }
            }
        }
        Ok(None) => Err(error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Task with id {} not found", id),
            request_id,
        )),
        Err(e) => {
            error!(request_id = %request_id, task_id = %id, error = %e, "Failed to get task for update");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve task: {}", e),
                request_id,
            ))
        }
    }
}

/// Cancel a task
/// POST /api/tasks/:id/cancel
pub async fn cancel_task(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Parse UUID from string
    let task_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(e) => {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_UUID",
                &format!("Invalid task ID format: {}", e),
                request_id,
            ));
        }
    };

    // Get existing task
    let task_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_task(&task_id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match task_result {
        Ok(Some(mut task)) => {
            // Check if task can be cancelled
            match &task.status {
                TaskStatus::Completed | TaskStatus::Failed(_) => {
                    return Err(error_response(
                        StatusCode::BAD_REQUEST,
                        "CANNOT_CANCEL",
                        &format!("Cannot cancel task with status: {:?}", task.status),
                        request_id,
                    ));
                }
                _ => {}
            }

            // Update status to failed with cancellation message
            task.status = TaskStatus::Failed("Task cancelled by user".to_string());
            task.updated_at = Utc::now();

            // Save updated task
            let task_clone = task.clone();
            let save_result = {
                let storage_guard = state.storage.read().await;
                if let Some(storage_manager) = storage_guard.as_ref() {
                    storage_manager.save_task(&task_clone).await
                } else {
                    Err(crate::error::Paper2CodesError::Storage(
                        crate::storage::errors::StorageError::NotConnected,
                    ))
                }
            };

            match save_result {
                Ok(_) => {
                    info!(request_id = %request_id, task_id = %id, "Cancelled task");

                    // Broadcast task cancellation via WebSocket
                    #[cfg(feature = "api")]
                    {
                        let _ = broadcast::broadcast_task_update(
                            &state,
                            &id,
                            "cancelled",
                            0.0,
                            Some("Task cancelled".to_string()),
                            None,
                        )
                        .await;
                    }

                    Ok(Json(ApiResponse::with_request_id(task, request_id)))
                }
                Err(e) => {
                    error!(request_id = %request_id, task_id = %id, error = %e, "Failed to cancel task");
                    Err(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "STORAGE_ERROR",
                        &format!("Failed to cancel task: {}", e),
                        request_id,
                    ))
                }
            }
        }
        Ok(None) => Err(error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            &format!("Task with id {} not found", id),
            request_id,
        )),
        Err(e) => {
            error!(request_id = %request_id, task_id = %id, error = %e, "Failed to get task for cancellation");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve task: {}", e),
                request_id,
            ))
        }
    }
}
