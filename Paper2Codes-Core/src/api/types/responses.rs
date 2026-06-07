use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "api")]

/// Standard API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub meta: ResponseMeta,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub timestamp: String,
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: Uuid::new_v4().to_string(),
            },
        }
    }

    pub fn with_request_id(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data,
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id,
            },
        }
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
    pub meta: ResponseMeta,
}

impl<T> PaginatedResponse<T> {
    pub fn with_request_id(
        data: Vec<T>,
        page: usize,
        per_page: usize,
        total: usize,
        request_id: String,
    ) -> Self {
        let total_pages = (total + per_page - 1) / per_page;

        Self {
            success: true,
            data,
            pagination: PaginationMeta {
                page,
                per_page,
                total,
                total_pages,
                next_cursor: None,
                prev_cursor: None,
                has_more: page < total_pages,
            },
            meta: ResponseMeta {
                timestamp: chrono::Utc::now().to_rfc3339(),
                request_id,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub total_pages: usize,
    /// Cursor for next page (cursor-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Cursor for previous page (cursor-based pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Whether there are more pages
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: usize, per_page: usize, total: usize) -> Self {
        let total_pages = (total + per_page - 1) / per_page;

        Self {
            success: true,
            data,
            pagination: PaginationMeta {
                page,
                per_page,
                total,
                total_pages,
                next_cursor: None,
                prev_cursor: None,
                has_more: page < total_pages,
            },
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: Uuid::new_v4().to_string(),
            },
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: String,
}

// ApiError is defined in errors.rs

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self::new(data)
    }
}

impl ApiResponse<()> {
    pub fn error(_error: crate::api::types::errors::ApiError) -> Self {
        ApiResponse {
            success: false,
            data: (),
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: Uuid::new_v4().to_string(),
            },
        }
    }
}

/// Paper processing status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStatusResponse {
    pub status: ProcessingStatus,
    pub progress: f32,
    pub message: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProcessingStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "parsing")]
    Parsing,
    #[serde(rename = "segmenting")]
    Segmenting,
    #[serde(rename = "extracting")]
    Extracting,
    #[serde(rename = "classifying")]
    Classifying,
    #[serde(rename = "embedding")]
    Embedding,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

/// Semantic search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub segment: crate::types::PaperSegment,
    pub score: f32,
    pub paper_id: String,
    pub paper_title: String,
}

/// Generation status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStatusResponse {
    pub status: GenerationStatus,
    pub progress: f32,
    pub repository_id: Option<String>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GenerationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "planning")]
    Planning,
    #[serde(rename = "analyzing")]
    Analyzing,
    #[serde(rename = "coding")]
    Coding,
    #[serde(rename = "verifying")]
    Verifying,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

/// Verification status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStatusResponse {
    pub status: VerificationStatus,
    pub progress: f32,
    pub report: Option<VerificationReport>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "static_analysis")]
    StaticAnalysis,
    #[serde(rename = "dynamic_testing")]
    DynamicTesting,
    #[serde(rename = "symbolic_verification")]
    SymbolicVerification,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub overall_score: f32,
    pub static_analysis: Option<StaticAnalysisResult>,
    pub dynamic_testing: Option<DynamicTestingResult>,
    pub symbolic_verification: Option<SymbolicVerificationResult>,
    pub issues: Vec<VerificationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAnalysisResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicTestingResult {
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub total_tests: usize,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicVerificationResult {
    pub verified_properties: usize,
    pub failed_properties: usize,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationIssue {
    pub severity: String,
    pub message: String,
    pub module_id: Option<String>,
    pub line_number: Option<usize>,
}
