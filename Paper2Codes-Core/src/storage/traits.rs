use crate::config::StorageConfig;
use crate::error::Result;
use crate::storage::filters::*;
use crate::storage::{ConnectionInfo, Document, GraphAnalysis, ModuleImpact, SearchResult};
use crate::types::*;
use async_trait::async_trait;

/// Storage trait for all storage operations
/// This trait defines the interface for all storage backends
#[async_trait]
pub trait Storage: Send + Sync {
    // Connection management
    async fn connect(&mut self, config: &StorageConfig) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
    async fn test_connection(&self) -> Result<()>;
    fn get_connection_info(&self) -> Option<ConnectionInfo>;
    async fn validate_schema(&self) -> Result<()> {
        Ok(())
    }

    // Papers
    async fn save_paper(&self, paper: &Paper) -> Result<()>;
    async fn get_paper(&self, id: &str) -> Result<Option<Paper>>;
    async fn list_papers(&self, filters: PaperFilters) -> Result<Vec<Paper>>;
    async fn search_papers(&self, query: &str) -> Result<Vec<Paper>>;
    async fn delete_paper(&self, id: &str) -> Result<()>;

    // Segments
    async fn save_segment(&self, segment: &PaperSegment) -> Result<()>;
    async fn get_segment(&self, id: &str) -> Result<Option<PaperSegment>>;
    async fn get_segments_by_paper(&self, paper_id: &str) -> Result<Vec<PaperSegment>>;
    async fn get_segments_by_ids(&self, ids: &[String]) -> Result<Vec<PaperSegment>>;

    // Repositories
    async fn save_repository(&self, repo: &Repository) -> Result<()>;
    async fn get_repository(&self, id: &str) -> Result<Option<Repository>>;
    async fn list_repositories(&self) -> Result<Vec<Repository>>;
    async fn update_repository(&self, repo: &Repository) -> Result<()>;
    async fn delete_repository(&self, id: &str) -> Result<()>;

    // Modules
    async fn save_module(&self, module: &CodeModule) -> Result<()>;
    async fn get_module(&self, id: &str) -> Result<Option<CodeModule>>;
    async fn get_modules_by_repository(&self, repo_id: &str) -> Result<Vec<CodeModule>>;
    async fn search_modules(&self, query: &str) -> Result<Vec<CodeModule>>;
    async fn delete_module(&self, id: &str) -> Result<()>;

    // Tasks
    async fn save_task(&self, task: &Task) -> Result<()>;
    async fn get_task(&self, id: &uuid::Uuid) -> Result<Option<Task>>;
    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>>;
    async fn get_tasks_by_paper(&self, paper_id: &str) -> Result<Vec<Task>>;
    async fn update_task_status(&self, id: &uuid::Uuid, status: TaskStatus) -> Result<()>;
    async fn delete_task(&self, id: &uuid::Uuid) -> Result<()>;

    // Dependencies (Graph)
    async fn add_dependency(&self, from: &str, to: &str) -> Result<()>;
    async fn remove_dependency(&self, from: &str, to: &str) -> Result<()>;
    async fn get_dependencies(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependents(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependency_chain(&self, module_id: &str) -> Result<Vec<String>>;
    async fn get_dependency_graph(&self, repo_id: &str) -> Result<DependencyGraph>;

    // Documents
    async fn save_document(&self, doc: &Document) -> Result<()>;
    async fn get_document(&self, id: &str) -> Result<Option<Document>>;
    async fn list_documents(&self, filters: DocumentFilters) -> Result<Vec<Document>>;
    async fn search_documents(&self, query: &str) -> Result<Vec<Document>>;
    async fn delete_document(&self, id: &str) -> Result<()>;

    // Vector operations (native SurrealDB)
    async fn save_embedding(&self, segment_id: &str, embedding: Vec<f32>) -> Result<()>;
    async fn get_embedding(&self, segment_id: &str) -> Result<Option<Vec<f32>>>;
    async fn vector_search(
        &self,
        query_embedding: Vec<f32>,
        k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>>;

    // Graph analysis
    async fn analyze_dependencies(&self, repo_id: &str) -> Result<GraphAnalysis>;
    async fn find_circular_dependencies(&self, repo_id: &str) -> Result<Vec<Vec<String>>>;
    async fn get_module_impact(&self, module_id: &str) -> Result<ModuleImpact>;

    // Users (for authentication)
    async fn save_user(&self, user: &crate::api::auth::user::User) -> Result<()>;
    async fn get_user(&self, id: &str) -> Result<Option<crate::api::auth::user::User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<crate::api::auth::user::User>>;
    async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::api::auth::user::User>>;
    async fn update_user(&self, user: &crate::api::auth::user::User) -> Result<()>;
    async fn delete_user(&self, id: &str) -> Result<()>;

    // Sessions (for refresh token management)
    #[cfg(feature = "api")]
    async fn save_session(&self, session: &crate::api::auth::user::Session) -> Result<()>;
    #[cfg(feature = "api")]
    async fn get_session(
        &self,
        refresh_token: &str,
    ) -> Result<Option<crate::api::auth::user::Session>>;
    async fn delete_session(&self, refresh_token: &str) -> Result<()>;
    #[cfg(feature = "api")]
    async fn delete_user_sessions(&self, user_id: &str) -> Result<()>;
    #[cfg(feature = "api")]
    async fn cleanup_expired_sessions(&self) -> Result<usize>;

    // Admin operations (optional - may not be supported by all backends)
    async fn execute_raw_query(&self, _query: &str) -> Result<serde_json::Value> {
        Err(crate::storage::errors::StorageError::NotSupported(
            "Raw query execution not supported".to_string(),
        )
        .into())
    }

    async fn get_database_stats(&self) -> Result<crate::storage::DatabaseStats> {
        Err(crate::storage::errors::StorageError::NotSupported(
            "Database statistics not supported".to_string(),
        )
        .into())
    }
}
