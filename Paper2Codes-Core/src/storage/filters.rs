use serde::{Deserialize, Serialize};

/// Filters for paper queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaperFilters {
    pub author: Option<String>,
    pub year: Option<u32>,
    pub keyword: Option<String>,
    pub venue: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filters for document queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentFilters {
    pub content_type: Option<String>,
    pub name: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filters for vector search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub paper_id: Option<String>,
    pub segment_type: Option<String>,
    pub min_score: Option<f32>,
}
