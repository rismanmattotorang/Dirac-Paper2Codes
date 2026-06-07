pub mod errors;
pub mod filters;
pub mod graph;
pub mod manager;
pub mod schema;
pub mod surreal;
pub mod traits;
pub mod vector;

#[cfg(test)]
mod tests;

pub use manager::StorageManager;

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_string: String,
    pub namespace: String,
    pub database: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Document type for general document storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Search result with similarity score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub segment: crate::types::PaperSegment,
    pub score: f32,
}

/// Graph analysis result
#[derive(Debug, Clone)]
pub struct GraphAnalysis {
    pub total_modules: usize,
    pub total_dependencies: usize,
    pub max_depth: usize,
    pub average_dependencies: f64,
    pub isolated_modules: Vec<String>,
}

/// Module impact analysis
#[derive(Debug, Clone)]
pub struct ModuleImpact {
    pub direct_dependents: usize,
    pub transitive_dependents: usize,
    pub direct_dependencies: usize,
    pub affected_modules: Vec<String>,
}

/// Database statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    pub paper_count: usize,
    pub segment_count: usize,
    pub repository_count: usize,
    pub module_count: usize,
    pub task_count: usize,
    pub dependency_count: usize,
    pub document_count: usize,
    pub total_records: usize,
}
