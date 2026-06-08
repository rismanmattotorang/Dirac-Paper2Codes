use crate::config::StorageConfig;
use crate::error::Result;
use crate::storage::errors::StorageError;
use crate::storage::surreal::SurrealStorage;
use crate::storage::ConnectionInfo;
use crate::storage::traits::Storage;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Storage manager for managing storage connections
#[derive(Clone)]
pub struct StorageManager {
    storage: Arc<RwLock<Option<Box<dyn Storage>>>>,
    config: Arc<RwLock<Option<StorageConfig>>>,
    connected_at: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(None)),
            connected_at: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to storage backend
    pub async fn connect(&self, config: StorageConfig) -> Result<()> {
        let mut storage_guard = self.storage.write().await;

        // Disconnect existing connection if any
        if let Some(ref mut storage) = *storage_guard {
            let _ = storage.disconnect().await;
        }

        // Create new SurrealDB storage
        let mut surreal_storage = SurrealStorage::new();
        surreal_storage.connect(&config).await?;

        *storage_guard = Some(Box::new(surreal_storage));
        *self.config.write().await = Some(config);
        *self.connected_at.write().await = Some(chrono::Utc::now());

        Ok(())
    }

    /// Disconnect from storage backend
    pub async fn disconnect(&self) -> Result<()> {
        let mut storage_guard = self.storage.write().await;

        if let Some(ref mut storage) = *storage_guard {
            storage.disconnect().await?;
        }

        *storage_guard = None;
        *self.config.write().await = None;
        *self.connected_at.write().await = None;

        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        let storage_guard = self.storage.read().await;
        storage_guard
            .as_ref()
            .map(|s| s.is_connected())
            .unwrap_or(false)
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.test_connection().await
    }

    pub async fn validate_schema(&self) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.validate_schema().await
    }

    /// Retrieve information about the active storage connection (if any)
    pub async fn connection_info(&self) -> Option<ConnectionInfo> {
        let storage_guard = self.storage.read().await;
        storage_guard
            .as_ref()
            .and_then(|storage| storage.get_connection_info())
    }

    /// Execute a storage operation with a closure
    pub async fn with_storage<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&dyn Storage) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        f(storage.as_ref()).await
    }

    /// Execute a mutable storage operation with a closure
    pub async fn with_storage_mut<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut dyn Storage) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let mut storage_guard = self.storage.write().await;
        let storage = storage_guard
            .as_mut()
            .ok_or_else(|| StorageError::NotConnected)?;
        f(storage.as_mut()).await
    }

    // Convenience methods for common operations
    pub async fn save_paper(&self, paper: &crate::types::Paper) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_paper(paper).await
    }

    pub async fn save_task(&self, task: &crate::types::Task) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_task(task).await
    }

    pub async fn save_module(&self, module: &crate::types::CodeModule) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_module(module).await
    }

    pub async fn get_module(&self, id: &str) -> Result<Option<crate::types::CodeModule>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_module(id).await
    }

    pub async fn get_modules_by_repository(
        &self,
        repo_id: &str,
    ) -> Result<Vec<crate::types::CodeModule>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_modules_by_repository(repo_id).await
    }

    pub async fn search_modules(&self, query: &str) -> Result<Vec<crate::types::CodeModule>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.search_modules(query).await
    }

    pub async fn save_repository(&self, repo: &crate::types::Repository) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_repository(repo).await
    }

    pub async fn save_segment(&self, segment: &crate::types::PaperSegment) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_segment(segment).await
    }

    pub async fn save_embedding(&self, segment_id: &str, embedding: Vec<f32>) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_embedding(segment_id, embedding).await
    }

    pub async fn save_document(&self, document: &crate::storage::Document) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_document(document).await
    }

    pub async fn get_document(&self, id: &str) -> Result<Option<crate::storage::Document>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_document(id).await
    }

    pub async fn update_repository(&self, repo: &crate::types::Repository) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.update_repository(repo).await
    }

    pub async fn update_task_status(
        &self,
        id: &uuid::Uuid,
        status: crate::types::TaskStatus,
    ) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.update_task_status(id, status).await
    }

    // Additional convenience methods for API handlers
    pub async fn get_paper(&self, id: &str) -> Result<Option<crate::types::Paper>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_paper(id).await
    }

    pub async fn list_papers(
        &self,
        filters: crate::storage::filters::PaperFilters,
    ) -> Result<Vec<crate::types::Paper>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_papers(filters).await
    }

    pub async fn delete_paper(&self, id: &str) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.delete_paper(id).await
    }

    pub async fn get_repository(&self, id: &str) -> Result<Option<crate::types::Repository>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_repository(id).await
    }

    pub async fn list_repositories(&self) -> Result<Vec<crate::types::Repository>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_repositories().await
    }

    pub async fn get_task(&self, id: &uuid::Uuid) -> Result<Option<crate::types::Task>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_task(id).await
    }

    pub async fn get_tasks_by_status(
        &self,
        status: crate::types::TaskStatus,
    ) -> Result<Vec<crate::types::Task>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_tasks_by_status(status).await
    }

    pub async fn get_tasks_by_paper(&self, paper_id: &str) -> Result<Vec<crate::types::Task>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_tasks_by_paper(paper_id).await
    }

    /// Get connection info
    pub async fn get_connection_info(&self) -> Option<crate::storage::ConnectionInfo> {
        let storage_guard = self.storage.read().await;
        let connected_at = *self.connected_at.read().await;
        storage_guard
            .as_ref()
            .and_then(|s| s.get_connection_info())
            .map(|mut info| {
                if let Some(connected_time) = connected_at {
                    info.connected_at = connected_time;
                }
                info
            })
    }

    /// Get configuration
    pub async fn config(&self) -> Option<StorageConfig> {
        self.config.read().await.clone()
    }

    /// Execute raw query (for admin console)
    pub async fn execute_raw_query(&self, query: &str) -> Result<serde_json::Value> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.execute_raw_query(query).await
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<crate::storage::DatabaseStats> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_database_stats().await
    }

    /// Get segments by paper ID
    pub async fn get_segments_by_paper(
        &self,
        paper_id: &str,
    ) -> Result<Vec<crate::types::PaperSegment>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.get_segments_by_paper(paper_id).await
    }

    // ---- Auth persistence (users / sessions) ----

    pub async fn save_user(&self, user: &crate::api::auth::user::User) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_user(user).await
    }

    pub async fn update_user(&self, user: &crate::api::auth::user::User) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.update_user(user).await
    }

    pub async fn list_users(&self) -> Result<Vec<crate::api::auth::user::User>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_users().await
    }

    #[cfg(feature = "api")]
    pub async fn save_session(&self, session: &crate::api::auth::user::Session) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_session(session).await
    }

    #[cfg(feature = "api")]
    pub async fn list_sessions(&self) -> Result<Vec<crate::api::auth::user::Session>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_sessions().await
    }

    #[cfg(feature = "api")]
    pub async fn delete_session(&self, refresh_token: &str) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.delete_session(refresh_token).await
    }

    #[cfg(feature = "api")]
    pub async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.delete_user_sessions(user_id).await
    }

    // ---- Durable job queue persistence ----

    pub async fn save_job(&self, job: &crate::jobs::Job) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_job(job).await
    }

    pub async fn list_jobs(&self) -> Result<Vec<crate::jobs::Job>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_jobs().await
    }

    // ---- Personal API token persistence ----

    #[cfg(feature = "api")]
    pub async fn save_api_token(&self, token: &crate::api::auth::tokens::ApiToken) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.save_api_token(token).await
    }

    #[cfg(feature = "api")]
    pub async fn list_api_tokens(&self) -> Result<Vec<crate::api::auth::tokens::ApiToken>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.list_api_tokens().await
    }

    #[cfg(feature = "api")]
    pub async fn delete_api_token(&self, id: &str) -> Result<()> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.delete_api_token(id).await
    }

    /// Perform vector search
    pub async fn vector_search(
        &self,
        query_embedding: Vec<f32>,
        k: usize,
        filters: Option<crate::storage::filters::SearchFilters>,
    ) -> Result<Vec<crate::storage::SearchResult>> {
        let storage_guard = self.storage.read().await;
        let storage = storage_guard
            .as_ref()
            .ok_or_else(|| StorageError::NotConnected)?;
        storage.vector_search(query_embedding, k, filters).await
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable backing for the job queue: write-through goes to SurrealDB, and the
/// queue rehydrates from `list_jobs` on startup.
#[async_trait::async_trait]
impl crate::jobs::JobStore for StorageManager {
    async fn persist_job(&self, job: &crate::jobs::Job) -> Result<()> {
        self.save_job(job).await
    }

    async fn load_jobs(&self) -> Result<Vec<crate::jobs::Job>> {
        self.list_jobs().await
    }
}

/// Durable backing for the auth stores: users/sessions persist to SurrealDB and
/// rehydrate on startup.
#[cfg(feature = "api")]
#[async_trait::async_trait]
impl crate::api::auth::store::AuthPersistence for StorageManager {
    async fn load_users(&self) -> Result<Vec<crate::api::auth::user::User>> {
        self.list_users().await
    }

    async fn save_user(&self, user: &crate::api::auth::user::User) -> Result<()> {
        StorageManager::save_user(self, user).await
    }

    async fn load_sessions(&self) -> Result<Vec<crate::api::auth::user::Session>> {
        self.list_sessions().await
    }

    async fn save_session(&self, session: &crate::api::auth::user::Session) -> Result<()> {
        StorageManager::save_session(self, session).await
    }

    async fn delete_session(&self, refresh_token: &str) -> Result<()> {
        StorageManager::delete_session(self, refresh_token).await
    }
}

/// Durable backing for personal API tokens.
#[cfg(feature = "api")]
#[async_trait::async_trait]
impl crate::api::auth::tokens::ApiTokenPersistence for StorageManager {
    async fn load_tokens(&self) -> Result<Vec<crate::api::auth::tokens::ApiToken>> {
        self.list_api_tokens().await
    }

    async fn save_token(&self, token: &crate::api::auth::tokens::ApiToken) -> Result<()> {
        StorageManager::save_api_token(self, token).await
    }

    async fn delete_token(&self, id: &str) -> Result<()> {
        StorageManager::delete_api_token(self, id).await
    }
}
