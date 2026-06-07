//! Module management handlers
//!
//! Handles module retrieval, updates, downloads, and verification

use crate::api::state::AppState;
use crate::api::types::responses::ApiResponse;
use crate::api::utils::{error_response, get_request_id};
use crate::types::{CodeModule, ProgrammingLanguage};
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{Json, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;
use zip::write::FileOptions;
use zip::CompressionMethod;

/// Module content response
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleContentResponse {
    pub id: String,
    pub repository_id: Option<String>,
    pub file_path: String,
    pub content: String,
    pub language: String,
    pub ast: Option<String>,
    pub dependencies: Vec<String>,
    pub tests: Vec<TestInfo>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestInfo {
    pub id: String,
    pub name: String,
    pub code: String,
    pub expected_output: Option<String>,
}

/// Module update request
#[derive(Debug, Deserialize)]
pub struct ModuleUpdateRequest {
    pub content: String,
    pub description: Option<String>,
}

/// Module verification response
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleVerificationResponse {
    pub module_id: String,
    pub status: String, // "passed", "failed", "warning", "pending"
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub tests_total: usize,
    pub coverage: f64,
    pub issues: Vec<VerificationIssue>,
    pub executed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub severity: String, // "error", "warning", "info"
    pub message: String,
    pub line: usize,
    pub column: Option<usize>,
}

/// Search results
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleSearchResult {
    pub id: String,
    pub file_path: String,
    pub language: String,
    pub score: f64,
    pub snippet: String,
}

/// Get modules for a repository
/// GET /api/repositories/:id/modules
pub async fn get_repository_modules(
    Extension(state): Extension<Arc<AppState>>,
    Path(repository_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<CodeModule>>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id.as_str(),
        ));
    }

    let repository_id_for_query = repository_id.clone();
    let (repository, modules) = state
        .with_storage(|storage| {
            let repository_id = repository_id_for_query.clone();
            async move {
                let repo = storage.get_repository(&repository_id).await?;
                let modules = storage.get_modules_by_repository(&repository_id).await?;
                Ok::<_, crate::error::Paper2CodesError>((repo, modules))
            }
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "Failed to load repository modules"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to load modules: {}", e),
                request_id.as_str(),
            )
        })?;

    let repository = match repository {
        Some(repo) => repo,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Repository with id {} not found", repository_id),
                request_id.as_str(),
            ))
        }
    };

    info!(
        request_id = %request_id,
        repository_id = %repository_id,
        module_count = modules.len(),
        "Retrieved repository modules"
    );

    // If storage returns modules without repository metadata (e.g. stale cache),
    // fall back to modules embedded on the repository object.
    let response_modules = if modules.is_empty() {
        repository.modules
    } else {
        modules
    }
    .into_iter()
    .collect();

    Ok(Json(ApiResponse::with_request_id(
        response_modules,
        request_id,
    )))
}

/// Get module content by ID
/// GET /api/modules/:id
pub async fn get_module_content(
    Extension(state): Extension<Arc<AppState>>,
    Path(module_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ModuleContentResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id.as_str(),
        ));
    }

    let module_fetch_id = module_id.clone();
    let module = state
        .with_storage(|storage| {
            let module_id = module_fetch_id.clone();
            async move { storage.get_module(&module_id).await }
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "Failed to load module {module_id}"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to load module: {}", e),
                request_id.as_str(),
            )
        })?;

    let module = match module {
        Some(module) => module,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Module with id {} not found", module_id),
                request_id.as_str(),
            ))
        }
    };

    let response = ModuleContentResponse {
        id: module.id.clone(),
        file_path: module.file_path.to_string_lossy().to_string(),
        repository_id: module.repository_id.clone(),
        content: module.content.clone(),
        language: language_to_string(&module.language),
        ast: module.ast.clone(),
        dependencies: module.dependencies.clone(),
        tests: module
            .tests
            .iter()
            .map(|test| TestInfo {
                id: test.id.clone(),
                name: test.name.clone(),
                code: test.code.clone(),
                expected_output: test.expected_output.clone(),
            })
            .collect(),
        status: status_to_string(&module.status),
        created_at: module.created_at.to_rfc3339(),
        updated_at: module.updated_at.to_rfc3339(),
    };

    Ok(Json(ApiResponse::with_request_id(response, request_id)))
}

/// Update module content
/// PUT /api/modules/:id
pub async fn update_module_content(
    Extension(state): Extension<Arc<AppState>>,
    Path(module_id): Path<String>,
    headers: HeaderMap,
    Json(update): Json<ModuleUpdateRequest>,
) -> Result<Json<ApiResponse<ModuleContentResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    info!(request_id = %request_id, module_id = %module_id, "Updating module content");

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id.as_str(),
        ));
    }

    let module_id_for_fetch = module_id.clone();
    let module = state
        .with_storage(|storage| {
            let module_id = module_id_for_fetch.clone();
            async move { storage.get_module(&module_id).await }
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "Failed to retrieve module {module_id} for update"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve module: {}", e),
                request_id.as_str(),
            )
        })?;

    let mut module = match module {
        Some(module) => module,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Module with id {} not found", module_id),
                request_id.as_str(),
            ))
        }
    };

    module.content = update.content.clone();
    module.updated_at = Utc::now();

    let module_to_save = module.clone();
    state
        .with_storage(|storage| async move {
            storage.save_module(&module_to_save).await?;
            Ok(())
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "Failed to persist module {module_id}"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to save module: {}", e),
                request_id.as_str(),
            )
        })?;

    let module = ModuleContentResponse {
        id: module.id.clone(),
        repository_id: module.repository_id.clone(),
        file_path: module.file_path.to_string_lossy().to_string(),
        content: module.content.clone(),
        language: language_to_string(&module.language),
        ast: module.ast.clone(),
        dependencies: module.dependencies.clone(),
        tests: module
            .tests
            .iter()
            .map(|test| TestInfo {
                id: test.id.clone(),
                name: test.name.clone(),
                code: test.code.clone(),
                expected_output: test.expected_output.clone(),
            })
            .collect(),
        status: status_to_string(&module.status),
        created_at: module.created_at.to_rfc3339(),
        updated_at: module.updated_at.to_rfc3339(),
    };

    Ok(Json(ApiResponse::with_request_id(module, request_id)))
}

/// Download module file
/// GET /api/modules/:id/download
pub async fn download_module(
    Path(module_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    info!("Module download request for: {}", module_id);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            Uuid::new_v4().to_string(),
        ));
    }

    let download_module_id = module_id.clone();
    let module = state
        .with_storage(|storage| {
            let module_id = download_module_id.clone();
            async move { storage.get_module(&module_id).await }
        })
        .await
        .map_err(|e| {
            error!(module_id = %module_id, error = %e, "Failed to fetch module for download");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to fetch module: {}", e),
                Uuid::new_v4().to_string(),
            )
        })?;

    let module = match module {
        Some(module) => module,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Module with id {} not found", module_id),
                Uuid::new_v4().to_string(),
            ));
        }
    };

    let file_name = module
        .file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("module-{}.txt", module_id));
    let file_content = module.content.into_bytes();
    let content_type = language_to_mime(&module.language);

    // Set appropriate headers
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("text/plain")),
    );
    headers.insert(
        "Content-Disposition",
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", file_name)).unwrap(),
    );
    headers.insert(
        "Content-Length",
        HeaderValue::from_str(&file_content.len().to_string()).unwrap(),
    );

    let mut response = Response::new(file_content.into());
    *response.status_mut() = StatusCode::OK;
    *response.headers_mut() = headers;
    Ok(response)
}

/// Download repository as ZIP
/// GET /api/repositories/:id/download
pub async fn download_repository(
    Path(repository_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    info!("Repository download request for: {}", repository_id);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            Uuid::new_v4().to_string(),
        ));
    }

    let repository_id_lookup = repository_id.clone();
    let (repository, modules) = state
        .with_storage(|storage| {
            let repository_id = repository_id_lookup.clone();
            async move {
                let repo = storage.get_repository(&repository_id).await?;
                let modules = storage.get_modules_by_repository(&repository_id).await?;
                Ok::<_, crate::error::Paper2CodesError>((repo, modules))
            }
        })
        .await
        .map_err(|e| {
            error!(
                repository_id = %repository_id,
                error = %e,
                "Failed to gather repository for download"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to load repository: {}", e),
                Uuid::new_v4().to_string(),
            )
        })?;

    let repository = match repository {
        Some(repo) => repo,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Repository {} not found", repository_id),
                Uuid::new_v4().to_string(),
            ));
        }
    };

    let modules = if modules.is_empty() {
        repository.modules
    } else {
        modules
    };

    if modules.is_empty() {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "NO_MODULES",
            &format!(
                "Repository {} does not contain any modules yet",
                repository_id
            ),
            Uuid::new_v4().to_string(),
        ));
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for module in modules {
            let path = module.file_path.to_string_lossy().to_string();
            if let Err(e) = zip.start_file(path.clone(), options) {
                error!("Failed to write module {} to zip: {}", path, e);
                return Err(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ZIP_WRITE_ERROR",
                    &format!("Failed to add {} to archive: {}", path, e),
                    Uuid::new_v4().to_string(),
                ));
            }

            if let Err(e) = zip.write_all(module.content.as_bytes()) {
                error!("Failed to write module {} contents to zip: {}", path, e);
                return Err(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ZIP_WRITE_ERROR",
                    &format!("Failed to write {}: {}", path, e),
                    Uuid::new_v4().to_string(),
                ));
            }
        }

        if let Err(e) = zip.finish() {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ZIP_FINALIZE_ERROR",
                &format!("Failed to finalize repository archive: {}", e),
                Uuid::new_v4().to_string(),
            ));
        }
    }

    let zip_content = cursor.into_inner();

    // Set appropriate headers
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/zip"));
    headers.insert(
        "Content-Disposition",
        HeaderValue::from_str(&format!(
            "attachment; filename=\"repository-{}.zip\"",
            repository_id
        ))
        .unwrap(),
    );
    headers.insert(
        "Content-Length",
        HeaderValue::from_str(&zip_content.len().to_string()).unwrap(),
    );

    let mut response = Response::new(zip_content.into());
    *response.status_mut() = StatusCode::OK;
    *response.headers_mut() = headers;
    Ok(response)
}

/// Get module verification results
/// GET /api/modules/:id/verification
pub async fn get_module_verification(
    Extension(state): Extension<Arc<AppState>>,
    Path(module_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ModuleVerificationResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    info!(request_id = %request_id, module_id = %module_id, "Getting module verification");

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id.as_str(),
        ));
    }

    let module_id_for_verification = module_id.clone();
    let module = state
        .with_storage(|storage| {
            let module_id = module_id_for_verification.clone();
            async move { storage.get_module(&module_id).await }
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                module_id = %module_id,
                error = %e,
                "Failed to load module for verification summary"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to load module: {}", e),
                request_id.clone(),
            )
        })?;

    let module = match module {
        Some(module) => module,
        None => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Module with id {} not found", module_id),
                request_id.as_str(),
            ))
        }
    };

    let total_tests = module.tests.len();

    let verification = ModuleVerificationResponse {
        module_id: module.id.clone(),
        status: if total_tests == 0 {
            "no_tests".to_string()
        } else {
            "pending".to_string()
        },
        tests_passed: 0,
        tests_failed: 0,
        tests_total: total_tests,
        coverage: if total_tests == 0 { 0.0 } else { 0.0 },
        issues: Vec::new(),
        executed_at: Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse::with_request_id(verification, request_id)))
}

/// Search modules
/// GET /api/modules/search
pub async fn search_modules(
    Extension(state): Extension<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ModuleSearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);
    let query = params.get("q").cloned().unwrap_or_default();

    info!(request_id = %request_id, query = %query, "Searching modules");

    if query.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            "Query parameter 'q' is required",
            request_id.as_str(),
        ));
    }

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id.as_str(),
        ));
    }

    let query_for_search = query.clone();
    let modules = state
        .with_storage(|storage| {
            let query = query_for_search.clone();
            async move { storage.search_modules(&query).await }
        })
        .await
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                "Module search failed"
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to search modules: {}", e),
                request_id.clone(),
            )
        })?;

    let lower_query = query.to_lowercase();
    let results: Vec<ModuleSearchResult> = modules
        .into_iter()
        .map(|module| {
            let content = module.content;
            let snippet = extract_snippet(&content, &lower_query);
            let score = compute_similarity(&content, &lower_query);
            ModuleSearchResult {
                id: module.id,
                file_path: module.file_path.to_string_lossy().to_string(),
                language: language_to_string(&module.language),
                score,
                snippet,
            }
        })
        .collect();

    Ok(Json(ApiResponse::with_request_id(results, request_id)))
}

fn language_to_string(language: &ProgrammingLanguage) -> String {
    match language {
        ProgrammingLanguage::Python => "python".to_string(),
        ProgrammingLanguage::Rust => "rust".to_string(),
        ProgrammingLanguage::Other(other) => other.clone(),
    }
}

fn status_to_string(status: &crate::types::ModuleStatus) -> String {
    match status {
        crate::types::ModuleStatus::Pending => "pending".to_string(),
        crate::types::ModuleStatus::Analyzing => "analyzing".to_string(),
        crate::types::ModuleStatus::Coding => "coding".to_string(),
        crate::types::ModuleStatus::Verifying => "verifying".to_string(),
        crate::types::ModuleStatus::Completed => "completed".to_string(),
        crate::types::ModuleStatus::Failed(reason) => format!("failed:{}", reason),
    }
}

fn language_to_mime(language: &ProgrammingLanguage) -> String {
    match language {
        ProgrammingLanguage::Python => "text/x-python".to_string(),
        ProgrammingLanguage::Rust => "text/x-rust".to_string(),
        ProgrammingLanguage::Other(other) => match other.to_lowercase().as_str() {
            "json" => "application/json".to_string(),
            "yaml" | "yml" => "application/x-yaml".to_string(),
            "markdown" | "md" => "text/markdown".to_string(),
            "xml" => "application/xml".to_string(),
            "txt" => "text/plain".to_string(),
            _ => "application/octet-stream".to_string(),
        },
    }
}

fn extract_snippet(content: &str, query_lower: &str) -> String {
    if content.is_empty() {
        return String::new();
    }

    let content_lower = content.to_lowercase();
    if let Some(idx) = content_lower.find(query_lower) {
        let start = idx.saturating_sub(80);
        let end = (idx + query_lower.len() + 80).min(content.len());
        let snippet = &content[start..end];
        snippet.replace('\n', " ")
    } else {
        content
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(160)
            .collect()
    }
}

fn compute_similarity(content: &str, query_lower: &str) -> f64 {
    if query_lower.is_empty() {
        return 0.0;
    }

    let content_lower = content.to_lowercase();
    let tokens: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.is_empty() {
        let occurrences = content_lower.matches(query_lower).count();
        let ratio = occurrences as f64 / (content_lower.len().max(query_lower.len()) as f64);
        return ratio.min(1.0);
    }

    let matches = tokens
        .iter()
        .filter(|token| content_lower.contains(**token))
        .count();
    (matches as f64 / tokens.len() as f64).min(1.0)
}
