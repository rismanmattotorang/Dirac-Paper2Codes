use serde::{Deserialize, Serialize};
#[cfg(feature = "api")]
use validator::Validate;

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_per_page")]
    pub per_page: usize,
    /// Cursor for cursor-based pagination (alternative to page-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// Fields to include in response (comma-separated, GraphQL-like)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
}

fn default_page() -> usize {
    1
}

fn default_per_page() -> usize {
    20
}

impl PaginationQuery {
    pub fn offset(&self) -> usize {
        (self.page - 1) * self.per_page
    }

    /// Validate pagination parameters
    pub fn validate_pagination(&self) -> Result<(), String> {
        // If cursor is provided, page-based pagination is ignored
        if self.cursor.is_some() {
            return Ok(());
        }

        if self.page == 0 {
            return Err("page must be greater than 0".to_string());
        }
        if self.per_page == 0 || self.per_page > 100 {
            return Err("per_page must be between 1 and 100".to_string());
        }
        Ok(())
    }

    /// Check if using cursor-based pagination
    pub fn is_cursor_based(&self) -> bool {
        self.cursor.is_some()
    }

    /// Parse field selection into a vector
    pub fn parse_fields(&self) -> Vec<String> {
        self.fields
            .as_ref()
            .map(|f| {
                f.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Create paper request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct CreatePaperRequest {
    #[cfg_attr(
        feature = "api",
        validate(length(
            min = 1,
            max = 500,
            message = "Title must be between 1 and 500 characters"
        ))
    )]
    pub title: String,
    #[cfg_attr(
        feature = "api",
        validate(length(max = 10000, message = "Abstract must be less than 10000 characters"))
    )]
    pub abstract_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Update paper request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct UpdatePaperRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "api",
        validate(length(
            min = 1,
            max = 500,
            message = "Title must be between 1 and 500 characters"
        ))
    )]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "api",
        validate(length(max = 10000, message = "Abstract must be less than 10000 characters"))
    )]
    pub abstract_text: Option<String>,
}

/// Semantic search request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct SemanticSearchRequest {
    #[cfg_attr(
        feature = "api",
        validate(length(
            min = 1,
            max = 1000,
            message = "Query must be between 1 and 1000 characters"
        ))
    )]
    pub query: String,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_type: Option<String>,
}

fn default_k() -> usize {
    10
}

/// Code generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(Validate))]
pub struct GenerateCodeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modules: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub generate_tests: bool,
}

fn default_true() -> bool {
    true
}
