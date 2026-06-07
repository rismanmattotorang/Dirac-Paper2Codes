//! File management handlers for Phase 5
//!
//! Handles file upload, download, preview, and versioning

use axum::{
    extract::{Extension, Multipart, Path},
    http::{HeaderValue, StatusCode},
    response::{Json, Response},
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::api::state::AppState;
use crate::api::types::responses::ApiResponse;
use crate::error::Result;
use crate::storage::errors::StorageError;
use crate::storage::Document;

/// File upload request
#[derive(Debug, Serialize, Deserialize)]
pub struct FileUploadRequest {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// File upload response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub file_id: String,
    pub name: String,
    pub size: usize,
    pub content_type: String,
    pub uploaded_at: String,
}

/// File metadata response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    pub size: usize,
    pub content_type: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub versions: Vec<FileVersion>,
    pub created_at: String,
    pub updated_at: String,
}

/// File version information
#[derive(Debug, Serialize, Deserialize)]
pub struct FileVersion {
    pub version: usize,
    pub size: usize,
    pub created_at: String,
    pub checksum: String,
}

/// Upload file with progress tracking
///
/// Endpoint: POST /api/files/upload
pub async fn upload_file(
    Extension(state): Extension<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<FileUploadResponse>>> {
    info!("File upload request received");

    if !state.config.storage.enabled {
        return Err(crate::error::Paper2CodesError::Storage(
            StorageError::NotConnected,
        ));
    }

    let mut file_name: Option<String> = None;
    let mut file_content: Vec<u8> = Vec::new();
    let mut content_type: String = "application/octet-stream".to_string();
    let mut description: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        crate::error::Paper2CodesError::Validation(format!(
            "Error parsing multipart/form-data request: {}",
            e
        ))
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                content_type = field
                    .content_type()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                // Read file content
                let data = field.bytes().await.map_err(|e| {
                    crate::error::Paper2CodesError::Validation(format!(
                        "Error reading file data: {}",
                        e
                    ))
                })?;
                file_content = data.to_vec();
            }
            "name" => {
                if let Ok(name) = field.text().await {
                    if file_name.is_none() {
                        file_name = Some(name);
                    }
                }
            }
            "description" => {
                if let Ok(desc) = field.text().await {
                    description = Some(desc);
                }
            }
            "tags" => {
                if let Ok(tags_str) = field.text().await {
                    tags = tags_str
                        .split(',')
                        .filter_map(|s| {
                            let trimmed = s.trim();
                            if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed.to_string())
                            }
                        })
                        .collect();
                }
            }
            _ => {}
        }
    }

    let file_name = file_name.ok_or_else(|| {
        crate::error::Paper2CodesError::Validation("File name is required".to_string())
    })?;

    // Generate file ID
    let file_id = Uuid::new_v4().to_string();

    let checksum = format!("{:x}", md5::compute(&file_content));
    let encoded_content = BASE64.encode(&file_content);
    let now = Utc::now();

    let metadata = serde_json::json!({
        "description": description,
        "tags": tags,
        "size": file_content.len(),
        "checksum": checksum,
        "versions": [
            {
                "version": 1,
                "size": file_content.len(),
                "checksum": checksum,
                "created_at": now.to_rfc3339()
            }
        ]
    });

    let document = Document {
        id: file_id.clone(),
        name: file_name.clone(),
        content: encoded_content,
        content_type: content_type.clone(),
        metadata,
        created_at: now,
        updated_at: now,
    };

    let document_to_store = document.clone();
    state
        .with_storage(|storage| {
            let document = document_to_store.clone();
            async move { storage.save_document(&document).await }
        })
        .await?;

    info!(
        file_id = %file_id,
        name = %file_name,
        size = file_content.len(),
        "File persisted to storage"
    );

    let response = FileUploadResponse {
        file_id: file_id.clone(),
        name: file_name.clone(),
        size: file_content.len(),
        content_type,
        uploaded_at: Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse::new(response)))
}

/// Download file
///
/// Endpoint: GET /api/files/:id
pub async fn download_file(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Response> {
    info!("File download request for: {}", file_id);

    if !state.config.storage.enabled {
        return Err(crate::error::Paper2CodesError::Storage(
            StorageError::NotConnected,
        ));
    }

    let download_id = file_id.clone();
    let document = state
        .with_storage(|storage| {
            let file_id = download_id.clone();
            async move { storage.get_document(&file_id).await }
        })
        .await?
        .ok_or_else(|| {
            crate::error::Paper2CodesError::Storage(StorageError::NotFound(format!(
                "File {} not found",
                file_id
            )))
        })?;

    let decoded = BASE64.decode(document.content.as_bytes()).map_err(|e| {
        crate::error::Paper2CodesError::Validation(format!("Invalid file content: {}", e))
    })?;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_str(&document.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        "Content-Length",
        HeaderValue::from_str(&decoded.len().to_string()).unwrap(),
    );

    if !document.name.is_empty() {
        if let Ok(disposition) =
            HeaderValue::from_str(&format!("attachment; filename=\"{}\"", document.name))
        {
            headers.insert("Content-Disposition", disposition);
        }
    }

    let mut response = Response::new(decoded.into());
    *response.status_mut() = StatusCode::OK;
    *response.headers_mut() = headers;
    Ok(response)
}

/// Preview file (for text-based files)
///
/// Endpoint: GET /api/files/:id/preview
pub async fn preview_file(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<ApiResponse<FilePreviewResponse>>> {
    info!("File preview request for: {}", file_id);

    if !state.config.storage.enabled {
        return Err(crate::error::Paper2CodesError::Storage(
            StorageError::NotConnected,
        ));
    }

    let preview_id = file_id.clone();
    let document = state
        .with_storage(|storage| {
            let file_id = preview_id.clone();
            async move { storage.get_document(&file_id).await }
        })
        .await?
        .ok_or_else(|| {
            crate::error::Paper2CodesError::Storage(StorageError::NotFound(format!(
                "File {} not found",
                file_id
            )))
        })?;

    let decoded = BASE64.decode(document.content.as_bytes()).map_err(|e| {
        crate::error::Paper2CodesError::Validation(format!("Invalid file content: {}", e))
    })?;

    let max_preview_bytes = 4096;
    let truncated = decoded.len() > max_preview_bytes;
    let preview_slice = if truncated {
        &decoded[..max_preview_bytes]
    } else {
        decoded.as_slice()
    };

    let (preview_type, content) = if is_text_content(&document.content_type) {
        let text = String::from_utf8_lossy(preview_slice).to_string();
        ("text".to_string(), text)
    } else if document.content_type.starts_with("image/") {
        (
            "image".to_string(),
            BASE64.encode(preview_slice), // client can render from base64
        )
    } else {
        ("binary".to_string(), BASE64.encode(preview_slice))
    };

    let preview = FilePreviewResponse {
        file_id: document.id,
        preview_type,
        content,
        truncated,
    };

    Ok(Json(ApiResponse::new(preview)))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilePreviewResponse {
    pub file_id: String,
    pub preview_type: String, // "text", "image", "code", etc.
    pub content: String,
    pub truncated: bool,
}

fn is_text_content(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || matches!(
            content_type,
            "application/json"
                | "application/xml"
                | "application/javascript"
                | "application/typescript"
                | "application/x-yaml"
                | "application/x-sh"
        )
}

fn parse_tags(metadata: &Value) -> Vec<String> {
    metadata
        .get("tags")
        .and_then(|tags| tags.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_versions(metadata: &Value, fallback_size: usize) -> Vec<FileVersion> {
    metadata
        .get("versions")
        .and_then(|versions| versions.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|version| {
                    let version_number = version.get("version")?.as_u64()? as usize;
                    let size = version
                        .get("size")
                        .and_then(|s| s.as_u64())
                        .map(|s| s as usize)
                        .unwrap_or(fallback_size);
                    let created_at = version
                        .get("created_at")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| Utc::now().to_rfc3339());
                    let checksum = version
                        .get("checksum")
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string();

                    Some(FileVersion {
                        version: version_number,
                        size,
                        created_at,
                        checksum,
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![FileVersion {
                version: 1,
                size: fallback_size,
                created_at: Utc::now().to_rfc3339(),
                checksum: metadata
                    .get("checksum")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
            }]
        })
}

fn build_file_metadata(document: &Document) -> Result<FileMetadata> {
    let tags = parse_tags(&document.metadata);
    let versions = parse_versions(&document.metadata, document.content.len());
    let size = document
        .metadata
        .get("size")
        .and_then(|s| s.as_u64())
        .map(|s| s as usize)
        .unwrap_or_else(|| {
            BASE64
                .decode(document.content.as_bytes())
                .map(|decoded| decoded.len())
                .unwrap_or(0)
        });

    let description = document
        .metadata
        .get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    Ok(FileMetadata {
        id: document.id.clone(),
        name: document.name.clone(),
        size,
        content_type: document.content_type.clone(),
        description,
        tags,
        versions,
        created_at: document.created_at.to_rfc3339(),
        updated_at: document.updated_at.to_rfc3339(),
    })
}

/// List file versions
///
/// Endpoint: GET /api/files/:id/versions
pub async fn list_file_versions(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<FileVersion>>>> {
    info!("List file versions request for: {}", file_id);

    if !state.config.storage.enabled {
        return Err(crate::error::Paper2CodesError::Storage(
            StorageError::NotConnected,
        ));
    }

    let versions_lookup_id = file_id.clone();
    let document = state
        .with_storage(|storage| {
            let file_id = versions_lookup_id.clone();
            async move { storage.get_document(&file_id).await }
        })
        .await?
        .ok_or_else(|| {
            crate::error::Paper2CodesError::Storage(StorageError::NotFound(format!(
                "File {} not found",
                file_id
            )))
        })?;

    let versions = parse_versions(&document.metadata, document.content.len());

    Ok(Json(ApiResponse::new(versions)))
}

/// Get file metadata
///
/// Endpoint: GET /api/files/:id/metadata
pub async fn get_file_metadata(
    Path(file_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<ApiResponse<FileMetadata>>> {
    info!("Get file metadata request for: {}", file_id);

    if !state.config.storage.enabled {
        return Err(crate::error::Paper2CodesError::Storage(
            StorageError::NotConnected,
        ));
    }

    let metadata_lookup_id = file_id.clone();
    let document = state
        .with_storage(|storage| {
            let file_id = metadata_lookup_id.clone();
            async move { storage.get_document(&file_id).await }
        })
        .await?
        .ok_or_else(|| {
            crate::error::Paper2CodesError::Storage(StorageError::NotFound(format!(
                "File {} not found",
                file_id
            )))
        })?;

    let metadata = build_file_metadata(&document)?;

    Ok(Json(ApiResponse::new(metadata)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Document;
    use chrono::Utc;

    #[test]
    fn parse_versions_uses_metadata_from_document() {
        let created_at = Utc::now();
        let metadata = serde_json::json!({
            "versions": [
                {
                    "version": 2,
                    "size": 42,
                    "created_at": created_at.to_rfc3339(),
                    "checksum": "abc123"
                }
            ]
        });

        let versions = parse_versions(&metadata, 10);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, 2);
        assert_eq!(versions[0].size, 42);
        assert_eq!(versions[0].created_at, created_at.to_rfc3339());
        assert_eq!(versions[0].checksum, "abc123");
    }

    #[test]
    fn build_file_metadata_falls_back_to_content_size() {
        let content_bytes = b"example file contents";
        let document = Document {
            id: "doc-1".to_string(),
            name: "example.txt".to_string(),
            content: BASE64.encode(content_bytes),
            content_type: "text/plain".to_string(),
            metadata: serde_json::json!({
                "description": "Sample file",
                "tags": ["alpha", "beta"]
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let metadata = build_file_metadata(&document).expect("metadata builds");
        assert_eq!(metadata.id, "doc-1");
        assert_eq!(metadata.name, "example.txt");
        assert!(
            metadata.size >= content_bytes.len(),
            "metadata size should reflect stored content length"
        );
        assert_eq!(metadata.description.as_deref(), Some("Sample file"));
        assert_eq!(metadata.tags, vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(metadata.versions.len(), 1);
        assert_eq!(metadata.versions[0].version, 1);
        assert_eq!(metadata.versions[0].size, document.content.len());
    }
}
