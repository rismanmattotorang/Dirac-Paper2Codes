use crate::api::state::AppState;
use crate::api::types::requests::SemanticSearchRequest;
use crate::api::types::responses::{
    ApiResponse, ProcessingStatus, ProcessingStatusResponse, SearchResult,
};
use crate::api::utils::{error_response, get_request_id, validation_error_response};
use crate::api::websocket::broadcast;
use crate::document::DocumentProcessor;
use crate::retrieval::embedding::EmbeddingService;
use crate::storage::filters::SearchFilters;
use crate::types::{Paper, PaperSegment};
/// Phase 3: Paper Processing API endpoints
/// This module implements the Phase 3 endpoints as specified in INTEGRATION.md
use axum::extract::{Extension, Multipart, Path};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, info, warn};
use uuid::Uuid;
#[cfg(feature = "api")]
use validator::Validate;

/// Upload paper file (PDF or text)
/// POST /api/papers/upload
pub async fn upload_paper(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
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

    // Create uploads directory if it doesn't exist
    let uploads_dir = PathBuf::from("uploads");
    if let Err(e) = fs::create_dir_all(&uploads_dir).await {
        error!(request_id = %request_id, error = %e, "Failed to create uploads directory");
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "STORAGE_ERROR",
            &format!("Failed to create uploads directory: {}", e),
            request_id,
        ));
    }

    let mut file_path: Option<PathBuf> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            &format!("Failed to read multipart field: {}", e),
            request_id.clone(),
        )
    })? {
        let name = field.name().unwrap_or("");

        if name == "file" || name == "paper" {
            let original_filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "upload.pdf".to_string());

            // Validate file extension
            let extension = PathBuf::from(&original_filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !["pdf", "txt", "text"].contains(&extension.as_str()) {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "INVALID_FILE_TYPE",
                    "Only PDF and text files are supported",
                    request_id,
                ));
            }

            // Generate unique filename
            let unique_filename = format!("{}_{}", Uuid::new_v4(), original_filename);
            let dest_path = uploads_dir.join(&unique_filename);

            // Save file
            let mut file = fs::File::create(&dest_path).await.map_err(|e| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "FILE_ERROR",
                    &format!("Failed to create file: {}", e),
                    request_id.clone(),
                )
            })?;

            let data = field.bytes().await.map_err(|e| {
                error_response(
                    StatusCode::BAD_REQUEST,
                    "INVALID_REQUEST",
                    &format!("Failed to read file data: {}", e),
                    request_id.clone(),
                )
            })?;

            // Check file size (max 50MB)
            const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
            if data.len() > MAX_FILE_SIZE {
                let _ = fs::remove_file(&dest_path).await;
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "FILE_TOO_LARGE",
                    &format!("File size exceeds maximum of {} bytes", MAX_FILE_SIZE),
                    request_id,
                ));
            }

            file.write_all(&data).await.map_err(|e| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "FILE_ERROR",
                    &format!("Failed to write file: {}", e),
                    request_id.clone(),
                )
            })?;

            file_path = Some(dest_path);
            info!(request_id = %request_id, filename = %original_filename, "Uploaded file");
        }
    }

    let file_path = file_path.ok_or_else(|| {
        error_response(
            StatusCode::BAD_REQUEST,
            "MISSING_FILE",
            "No file provided in upload",
            request_id.clone(),
        )
    })?;

    // Parse the paper based on file extension
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let processor = DocumentProcessor::new();
    let paper_result = if extension == "pdf" {
        processor.parse_pdf(&file_path).await
    } else {
        // For text files, read content and parse
        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let filename = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());
                processor.parse_text(&content, filename).await
            }
            Err(e) => {
                let _ = fs::remove_file(&file_path).await;
                error!(request_id = %request_id, error = %e, "Failed to read text file");
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "FILE_READ_ERROR",
                    &format!("Failed to read file: {}", e),
                    request_id,
                ));
            }
        }
    };

    // Update file path in metadata if parsing succeeded
    let paper = match paper_result {
        Ok(mut p) => {
            p.metadata.file_path = Some(file_path.clone());
            p
        }
        Err(e) => {
            // Clean up uploaded file on error
            let _ = fs::remove_file(&file_path).await;
            error!(request_id = %request_id, error = %e, "Failed to parse paper");
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "PARSE_ERROR",
                &format!("Failed to parse paper: {}", e),
                request_id,
            ));
        }
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
            info!(request_id = %request_id, paper_id = %paper_id, "Uploaded and saved paper");
            Ok(Json(ApiResponse::with_request_id(paper, request_id)))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to save uploaded paper");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to save paper: {}", e),
                request_id,
            ))
        }
    }
}

/// Process a paper (parse, segment, extract, classify, generate embeddings)
/// POST /api/papers/:id/process
pub async fn process_paper(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ProcessingStatusResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Get paper from storage
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

    let paper = match paper_result {
        Ok(Some(paper)) => paper,
        Ok(None) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Paper with id {} not found", id),
                request_id,
            ));
        }
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to get paper");
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve paper: {}", e),
                request_id,
            ));
        }
    };

    // Start async processing in background
    let state_clone = state.clone();
    let paper_id = id.clone();
    let request_id_clone = request_id.clone();

    // Broadcast initial processing started message
    #[cfg(feature = "api")]
    {
        let _ = broadcast::broadcast_paper_processed(
            &state,
            &paper_id,
            "processing",
            0,
            Some("Paper processing started".to_string()),
        )
        .await;
    }

    tokio::spawn(async move {
        let _ = process_paper_background(state_clone, paper, paper_id, request_id_clone).await;
    });

    // Return immediately with pending status
    let status = ProcessingStatusResponse {
        status: ProcessingStatus::Pending,
        progress: 0.0,
        message: Some("Processing started".to_string()),
        error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    info!(request_id = %request_id, paper_id = %id, "Started paper processing");
    Ok(Json(ApiResponse::with_request_id(status, request_id)))
}

/// Background processing task
async fn process_paper_background(
    state: Arc<AppState>,
    mut paper: Paper,
    paper_id: String,
    request_id: String,
) -> crate::error::Result<()> {
    use crate::domain::detector::DomainDetector;

    info!(request_id = %request_id, paper_id = %paper_id, "Starting background paper processing");

    // Create document processor with services
    let domain_detector = Arc::new(DomainDetector::new());
    let embedding_service = {
        // Try to get OpenAI API key from providers
        let api_key = state
            .config
            .llm
            .providers
            .get("openai")
            .or_else(|| {
                state
                    .config
                    .llm
                    .providers
                    .values()
                    .find(|p| p.enabled && p.api_key.is_some())
            })
            .and_then(|p| p.api_key.clone());

        if let Some(api_key) = api_key {
            match EmbeddingService::new_openai(api_key) {
                Ok(service) => Some(Arc::new(service)),
                Err(e) => {
                    error!(request_id = %request_id, error = %e, "Failed to create embedding service");
                    None
                }
            }
        } else {
            None
        }
    };

    // Create processor without storage (will handle storage separately)
    let processor = DocumentProcessor::with_services(
        Some(domain_detector),
        embedding_service.clone(),
        None, // Storage will be accessed through state
        None,
    );

    // Process paper (this will segment, extract, classify, and generate embeddings)
    // Note: process_paper_full requires storage, so we'll do it step by step

    // Broadcast segmenting status
    #[cfg(feature = "api")]
    {
        let _ = broadcast::broadcast_paper_processed(
            &state,
            &paper_id,
            "segmenting",
            0,
            Some("Segmenting paper content".to_string()),
        )
        .await;
    }

    processor.segment_paper(&mut paper)?;

    // Broadcast extracting status
    #[cfg(feature = "api")]
    {
        let _ = broadcast::broadcast_paper_processed(
            &state,
            &paper_id,
            "extracting",
            paper.segments.len(),
            Some(format!(
                "Extracting algorithms and equations from {} segments",
                paper.segments.len()
            )),
        )
        .await;
    }

    paper.algorithms = processor.extract_algorithms(&paper);
    paper.equations = processor.extract_equations(&paper);

    // Generate embeddings if service is available
    if let Some(ref embedding_service) = embedding_service {
        // Broadcast embedding status
        #[cfg(feature = "api")]
        {
            let _ = broadcast::broadcast_paper_processed(
                &state,
                &paper_id,
                "embedding",
                paper.segments.len(),
                Some(format!(
                    "Generating embeddings for {} segments",
                    paper.segments.len()
                )),
            )
            .await;
        }

        // Generate embeddings in batches to avoid overwhelming the API
        const BATCH_SIZE: usize = 10;
        let texts: Vec<String> = paper.segments.iter().map(|s| s.content.clone()).collect();

        // Process embeddings in batches with improved error handling
        let total_batches = (texts.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        let mut successful_embeddings = 0;

        for (batch_idx, batch) in texts.chunks(BATCH_SIZE).enumerate() {
            match embedding_service.embed_batch(batch).await {
                Ok(embeddings) => {
                    let start_idx = batch_idx * BATCH_SIZE;
                    for (idx, embedding) in embeddings.iter().enumerate() {
                        if let Some(segment) = paper.segments.get_mut(start_idx + idx) {
                            segment.embedding = Some(embedding.clone());
                            successful_embeddings += 1;
                        }
                    }

                    // Broadcast progress update
                    #[cfg(feature = "api")]
                    {
                        let _ = broadcast::broadcast_paper_processed(
                            &state,
                            &paper_id,
                            "embedding",
                            successful_embeddings,
                            Some(format!(
                                "Generated embeddings for {}/{} segments (batch {}/{})",
                                successful_embeddings,
                                paper.segments.len(),
                                batch_idx + 1,
                                total_batches
                            )),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    error!(request_id = %request_id, paper_id = %paper_id, batch = batch_idx, error = %e, "Failed to generate embeddings for batch");
                    // Continue with other batches even if one fails
                    // Log warning but don't fail the entire process
                    warn!(request_id = %request_id, paper_id = %paper_id, batch = batch_idx, "Skipping failed embedding batch, continuing with remaining batches");
                }
            }
        }

        info!(request_id = %request_id, paper_id = %paper_id, successful = successful_embeddings, total = paper.segments.len(), "Completed embedding generation");
    }

    // Save updated paper and segments to storage
    let save_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            // Save paper
            let paper_clone = paper.clone();
            let save_paper_result = storage_manager.save_paper(&paper_clone).await;

            if save_paper_result.is_err() {
                save_paper_result
            } else {
                // Save segments with embeddings (continue even if some fail)
                let mut segment_errors = 0;
                for (idx, segment) in paper.segments.iter().enumerate() {
                    if let Err(e) = storage_manager.save_segment(segment).await {
                        error!(request_id = %request_id, paper_id = %paper_id, segment_idx = idx, error = %e, "Failed to save segment");
                        segment_errors += 1;
                        // Continue saving other segments
                    }

                    // Save embedding if available
                    if let Some(ref embedding) = segment.embedding {
                        if let Err(e) = storage_manager
                            .save_embedding(&segment.id, embedding.clone())
                            .await
                        {
                            error!(request_id = %request_id, paper_id = %paper_id, segment_id = %segment.id, error = %e, "Failed to save embedding");
                            segment_errors += 1;
                            // Continue saving other embeddings
                        }
                    }
                }

                // If all segments failed, return error; otherwise log warnings but continue
                if segment_errors == paper.segments.len() && !paper.segments.is_empty() {
                    Err(crate::error::Paper2CodesError::Storage(
                        crate::storage::errors::StorageError::NotConnected,
                    ))
                } else if segment_errors > 0 {
                    warn!(request_id = %request_id, paper_id = %paper_id, failed_segments = segment_errors, total_segments = paper.segments.len(), "Some segments failed to save");
                    Ok(())
                } else {
                    Ok(())
                }
            }
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match save_result {
        Ok(_) => {
            info!(request_id = %request_id, paper_id = %paper_id, segments = paper.segments.len(), "Completed paper processing");

            // Broadcast completion message
            #[cfg(feature = "api")]
            {
                let _ = broadcast::broadcast_paper_processed(
                    &state,
                    &paper_id,
                    "completed",
                    paper.segments.len(),
                    Some(format!(
                        "Paper processed successfully with {} segments",
                        paper.segments.len()
                    )),
                )
                .await;
            }

            Ok(())
        }
        Err(e) => {
            error!(request_id = %request_id, paper_id = %paper_id, error = %e, "Failed to save processed paper");

            // Broadcast error message
            #[cfg(feature = "api")]
            {
                let _ = broadcast::broadcast_paper_processed(
                    &state,
                    &paper_id,
                    "failed",
                    0,
                    Some(format!("Paper processing failed: {}", e)),
                )
                .await;
            }

            Err(e)
        }
    }
}

/// Get paper processing status
/// GET /api/papers/:id/status
pub async fn get_processing_status(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ProcessingStatusResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    // Get paper to check processing status
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

    let paper = match paper_result {
        Ok(Some(paper)) => paper,
        Ok(None) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Paper with id {} not found", id),
                request_id,
            ));
        }
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to get paper");
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve paper: {}", e),
                request_id,
            ));
        }
    };

    // Determine status based on paper state with more precise logic
    let segments_with_embeddings = paper
        .segments
        .iter()
        .filter(|s| s.embedding.is_some())
        .count();
    let total_segments = paper.segments.len();

    let status = if total_segments > 0 && segments_with_embeddings == total_segments {
        ProcessingStatus::Completed
    } else if total_segments > 0 && segments_with_embeddings > 0 {
        ProcessingStatus::Embedding
    } else if total_segments > 0 {
        ProcessingStatus::Extracting
    } else if !paper.algorithms.is_empty() || !paper.equations.is_empty() {
        ProcessingStatus::Extracting
    } else if !paper.title.is_empty() {
        ProcessingStatus::Segmenting
    } else {
        ProcessingStatus::Pending
    };

    // Calculate more accurate progress based on actual completion
    let progress = if total_segments > 0 {
        // Progress based on segments with embeddings
        segments_with_embeddings as f64 / total_segments as f64
    } else {
        match status {
            ProcessingStatus::Completed => 1.0,
            ProcessingStatus::Embedding => 0.8,
            ProcessingStatus::Extracting => 0.6,
            ProcessingStatus::Segmenting => 0.4,
            ProcessingStatus::Classifying => 0.3,
            ProcessingStatus::Parsing => 0.2,
            ProcessingStatus::Pending => 0.0,
            ProcessingStatus::Failed => 0.0,
        }
    };

    let status_response = ProcessingStatusResponse {
        status,
        progress: progress as f32, // Convert f64 to f32 for API response
        message: Some(format!("Paper has {} segments", paper.segments.len())),
        error: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse::with_request_id(
        status_response,
        request_id,
    )))
}

/// Get paper segments
/// GET /api/papers/:id/segments
pub async fn get_paper_segments(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<PaperSegment>>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    if !state.config.storage.enabled {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_NOT_ENABLED",
            "Storage is not enabled",
            request_id,
        ));
    }

    // Get segments by paper ID
    let segments_result = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager.get_segments_by_paper(&id).await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match segments_result {
        Ok(segments) => {
            info!(request_id = %request_id, paper_id = %id, count = segments.len(), "Retrieved paper segments");
            Ok(Json(ApiResponse::with_request_id(segments, request_id)))
        }
        Err(e) => {
            error!(request_id = %request_id, paper_id = %id, error = %e, "Failed to get segments");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                &format!("Failed to retrieve segments: {}", e),
                request_id,
            ))
        }
    }
}

/// Semantic search papers
/// POST /api/papers/search
pub async fn search_papers(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<SemanticSearchRequest>,
) -> Result<Json<ApiResponse<Vec<SearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
    let request_id = get_request_id(&headers);

    // Validate request
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

    // Generate query embedding
    let api_key = state
        .config
        .llm
        .providers
        .get("openai")
        .or_else(|| {
            state
                .config
                .llm
                .providers
                .values()
                .find(|p| p.enabled && p.api_key.is_some())
        })
        .and_then(|p| p.api_key.clone());

    let embedding_service = if let Some(api_key) = api_key {
        match EmbeddingService::new_openai(api_key) {
            Ok(service) => Arc::new(service),
            Err(e) => {
                error!(request_id = %request_id, error = %e, "Failed to create embedding service");
                return Err(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "EMBEDDING_ERROR",
                    &format!("Failed to initialize embedding service: {}", e),
                    request_id,
                ));
            }
        }
    } else {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "EMBEDDING_NOT_CONFIGURED",
            "Embedding service is not configured",
            request_id,
        ));
    };

    let query_embedding = match embedding_service.embed(&request.query).await {
        Ok(embedding) => embedding,
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to generate query embedding");
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EMBEDDING_ERROR",
                &format!("Failed to generate embedding: {}", e),
                request_id,
            ));
        }
    };

    // Build search filters
    let paper_id_clone = request.paper_id.clone();
    let filters = Some(SearchFilters {
        paper_id: request.paper_id,
        segment_type: request.segment_type,
        min_score: None,
    });

    // Perform vector search
    let search_results = {
        let storage_guard = state.storage.read().await;
        if let Some(storage_manager) = storage_guard.as_ref() {
            storage_manager
                .vector_search(query_embedding, request.k, filters)
                .await
        } else {
            Err(crate::error::Paper2CodesError::Storage(
                crate::storage::errors::StorageError::NotConnected,
            ))
        }
    };

    match search_results {
        Ok(results) => {
            // Convert to SearchResult format
            // Use paper_id from request filter if available (all results will be from that paper)
            let paper_id_from_request = paper_id_clone
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            // Get paper title if we have paper_id
            let paper_title = if paper_id_from_request != "unknown" {
                let paper_result = {
                    let storage_guard = state.storage.read().await;
                    if let Some(storage_manager) = storage_guard.as_ref() {
                        let pid = paper_id_from_request.clone();
                        storage_manager.get_paper(&pid).await
                    } else {
                        Err(crate::error::Paper2CodesError::Storage(
                            crate::storage::errors::StorageError::NotConnected,
                        ))
                    }
                };

                if let Ok(Some(paper)) = paper_result {
                    paper.title
                } else {
                    "Unknown Paper".to_string()
                }
            } else {
                "Unknown Paper".to_string()
            };

            let search_results: Vec<SearchResult> = results
                .into_iter()
                .map(|result| SearchResult {
                    segment: result.segment,
                    score: result.score,
                    paper_id: paper_id_from_request.clone(),
                    paper_title: paper_title.clone(),
                })
                .collect();

            info!(request_id = %request_id, query = %request.query, count = search_results.len(), "Performed semantic search");
            Ok(Json(ApiResponse::with_request_id(
                search_results,
                request_id,
            )))
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to perform semantic search");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SEARCH_ERROR",
                &format!("Failed to perform search: {}", e),
                request_id,
            ))
        }
    }
}
