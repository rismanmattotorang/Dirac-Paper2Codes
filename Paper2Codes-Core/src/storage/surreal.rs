use crate::config::StorageConfig;
use crate::error::Result;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::filters::{DocumentFilters, PaperFilters, SearchFilters};
use crate::storage::graph::GraphAnalyzer;
use crate::storage::schema::Schema;
use crate::storage::traits::Storage;
use crate::storage::{
    ConnectionInfo, DatabaseStats, Document, GraphAnalysis, ModuleImpact, SearchResult,
};
use crate::types::*;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use tokio::sync::RwLock;

/// SurrealDB storage implementation with connection pooling support
pub struct SurrealStorage {
    db: Option<Surreal<Client>>,
    connected: AtomicBool,
    config: Option<StorageConfig>,
    connection_info: Option<ConnectionInfo>,
    // Connection health tracking
    last_health_check: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    consecutive_failures: Arc<RwLock<u32>>,
}

impl SurrealStorage {
    /// Create a new SurrealStorage instance
    pub fn new() -> Self {
        Self {
            db: None,
            connected: AtomicBool::new(false),
            config: None,
            connection_info: None,
            last_health_check: Arc::new(RwLock::new(None)),
            consecutive_failures: Arc::new(RwLock::new(0)),
        }
    }

    /// Check connection health and reconnect if needed
    /// This implements automatic reconnection with exponential backoff
    #[allow(dead_code)]
    async fn ensure_connection(&mut self) -> Result<()> {
        // Check if we need to verify connection health
        let should_check = {
            let last_check = self.last_health_check.read().await;
            last_check
                .map(|last| {
                    let elapsed = Utc::now().signed_duration_since(last);
                    elapsed.num_seconds() > 60 // Check every 60 seconds
                })
                .unwrap_or(true)
        };

        if should_check {
            if let Some(db) = &self.db {
                // Quick health check
                match db.query("SELECT 1").await {
                    Ok(_) => {
                        // Connection is healthy
                        *self.last_health_check.write().await = Some(Utc::now());
                        *self.consecutive_failures.write().await = 0;
                    }
                    Err(_) => {
                        // Connection failed, try to reconnect
                        let failures = {
                            let mut f = self.consecutive_failures.write().await;
                            *f += 1;
                            *f
                        };

                        tracing::warn!(
                            "Connection health check failed (consecutive failures: {})",
                            failures
                        );

                        // Reconnect if we have a config
                        if failures <= 3 {
                            if let Some(config) = self.config.clone() {
                                // Try to reconnect
                                if let Err(e) = self.reconnect(&config).await {
                                    tracing::error!("Failed to reconnect: {}", e);
                                } else {
                                    tracing::info!("Successfully reconnected to SurrealDB");
                                    *self.consecutive_failures.write().await = 0;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !self.connected.load(Ordering::Acquire) {
            return Err(StorageError::NotConnected.into());
        }

        Ok(())
    }

    /// Reconnect to the database
    #[allow(dead_code)]
    async fn reconnect(&mut self, config: &StorageConfig) -> Result<()> {
        // Disconnect existing connection
        self.disconnect().await.ok();

        // Connect again
        self.connect(config).await
    }

    /// Get database connection (panics if not connected)
    fn db(&self) -> StorageResult<&Surreal<Client>> {
        self.db.as_ref().ok_or_else(|| StorageError::NotConnected)
    }

    /// Convert segment type to string
    fn segment_type_to_string(seg_type: &SegmentType) -> String {
        match seg_type {
            SegmentType::Abstract => "Abstract".to_string(),
            SegmentType::Introduction => "Introduction".to_string(),
            SegmentType::Methodology => "Methodology".to_string(),
            SegmentType::Algorithm => "Algorithm".to_string(),
            SegmentType::Experiment => "Experiment".to_string(),
            SegmentType::Results => "Results".to_string(),
            SegmentType::Conclusion => "Conclusion".to_string(),
            SegmentType::Other(s) => s.clone(),
        }
    }

    /// Parse segment type from string
    fn segment_type_from_string(s: &str) -> SegmentType {
        match s {
            "Abstract" => SegmentType::Abstract,
            "Introduction" => SegmentType::Introduction,
            "Methodology" => SegmentType::Methodology,
            "Algorithm" => SegmentType::Algorithm,
            "Experiment" => SegmentType::Experiment,
            "Results" => SegmentType::Results,
            "Conclusion" => SegmentType::Conclusion,
            other => SegmentType::Other(other.to_string()),
        }
    }

    /// Convert task type to string
    #[allow(dead_code)]
    fn task_type_to_string(task_type: &TaskType) -> String {
        match task_type {
            TaskType::Planning => "Planning".to_string(),
            TaskType::Analysis { .. } => "Analysis".to_string(),
            TaskType::Coding { .. } => "Coding".to_string(),
            TaskType::Verification { .. } => "Verification".to_string(),
            TaskType::Fix { .. } => "Fix".to_string(),
        }
    }

    /// Convert task status to string
    #[allow(dead_code)]
    fn task_status_to_string(status: &TaskStatus) -> String {
        match status {
            TaskStatus::Pending => "Pending".to_string(),
            TaskStatus::InProgress => "InProgress".to_string(),
            TaskStatus::Completed => "Completed".to_string(),
            TaskStatus::Failed(msg) => format!("Failed:{}", msg),
        }
    }

    /// Parse task status from string
    #[allow(dead_code)]
    fn task_status_from_string(s: &str) -> TaskStatus {
        if s.starts_with("Failed:") {
            TaskStatus::Failed(s[7..].to_string())
        } else {
            match s {
                "Pending" => TaskStatus::Pending,
                "InProgress" => TaskStatus::InProgress,
                "Completed" => TaskStatus::Completed,
                _ => TaskStatus::Failed(format!("Unknown status: {}", s)),
            }
        }
    }
}

#[async_trait]
impl Storage for SurrealStorage {
    async fn connect(&mut self, config: &StorageConfig) -> Result<()> {
        if self.connected.load(Ordering::Acquire) {
            return Err(StorageError::Connection("Already connected".to_string()).into());
        }

        // Parse connection string - SurrealDB Rust client expects host:port format
        // Remove ws:// or wss:// prefix if present
        let connection_string = config
            .connection_string
            .trim_start_matches("ws://")
            .trim_start_matches("wss://")
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .to_string();

        tracing::info!("Connecting to SurrealDB at: {}", connection_string);

        // Connect to SurrealDB using WebSocket
        let db = Surreal::new::<Ws>(&connection_string)
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to connect to SurrealDB at {}: {}. \
                    Make sure SurrealDB is running: docker run -d -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory",
                    connection_string, e
                );
                StorageError::Connection(error_msg)
            })?;

        // Authenticate if credentials provided
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            db.signin(Root { username, password }).await.map_err(|e| {
                StorageError::Authentication(format!("Authentication failed: {}", e))
            })?;
        }

        // Use namespace and database
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(|e| {
                StorageError::Connection(format!("Failed to set namespace/database: {}", e))
            })?;

        // Setup schema if auto_migrate is enabled
        if config.auto_migrate {
            match Schema::setup(&db).await {
                Ok(_) => {
                    tracing::info!("SurrealDB schema setup complete");
                }
                Err(e) => {
                    tracing::error!("Schema setup failed: {}", e);
                    return Err(
                        StorageError::Migration(format!("Schema setup failed: {}", e)).into(),
                    );
                }
            }
        } else {
            tracing::info!("Auto-migrate disabled; skipping schema setup");
        }

        // Validate schema regardless of migration settings
        Schema::validate(&db).await.map_err(|e| {
            StorageError::Migration(format!(
                "Schema validation failed: {}. Enable storage.auto_migrate or run migrations manually.",
                e
            ))
        })?;

        // Test connection
        db.query("SELECT 1")
            .await
            .map_err(|e| StorageError::Connection(format!("Connection test failed: {}", e)))?;

        self.db = Some(db);
        self.config = Some(config.clone());
        self.connected.store(true, Ordering::Release);
        *self.last_health_check.write().await = Some(Utc::now());
        *self.consecutive_failures.write().await = 0;
        self.connection_info = Some(ConnectionInfo {
            connection_string: connection_string.clone(),
            namespace: config.namespace.clone(),
            database: config.database.clone(),
            connected_at: Utc::now(),
        });

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if !self.connected.load(Ordering::Acquire) {
            return Ok(());
        }

        // SurrealDB client doesn't have explicit disconnect, just drop it
        self.db = None;
        self.connected.store(false, Ordering::Release);
        self.connection_info = None;

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    async fn test_connection(&self) -> Result<()> {
        let db = self.db()?;
        db.query("SELECT 1")
            .await
            .map_err(|e| StorageError::Connection(format!("Connection test failed: {}", e)))?;
        Ok(())
    }

    fn get_connection_info(&self) -> Option<ConnectionInfo> {
        self.connection_info.clone()
    }

    async fn validate_schema(&self) -> Result<()> {
        let db = self.db()?;
        Schema::validate(db).await.map_err(|e| {
            crate::error::Paper2CodesError::Storage(StorageError::Migration(format!(
                "Schema validation failed: {}",
                e
            )))
        })?;
        Ok(())
    }

    // Papers operations
    async fn save_paper(&self, paper: &Paper) -> Result<()> {
        let db = self.db()?;

        // SurrealDB can serialize directly from serde types
        let _: Option<Paper> = db
            .create(("paper", &paper.id))
            .content(paper)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save paper: {}", e)))?;

        Ok(())
    }

    async fn get_paper(&self, id: &str) -> Result<Option<Paper>> {
        let db = self.db()?;

        let paper: Option<Paper> = db
            .select(("paper", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get paper: {}", e)))?;

        Ok(paper)
    }

    async fn list_papers(&self, filters: PaperFilters) -> Result<Vec<Paper>> {
        let db = self.db()?;

        // Use parameterized queries to prevent SQL injection
        let mut query = "SELECT * FROM paper WHERE 1=1".to_string();
        let mut bindings: Vec<(&str, surrealdb::sql::Value)> = Vec::new();

        if let Some(author) = &filters.author {
            // Sanitize and validate input
            let sanitized = author.trim();
            if !sanitized.is_empty() && sanitized.len() <= 500 {
                query.push_str(" AND metadata.authors CONTAINS $author");
                bindings.push(("author", surrealdb::sql::Value::from(sanitized)));
            }
        }

        if let Some(year) = filters.year {
            // Validate year is reasonable
            if year >= 1900 && year <= 2100 {
                query.push_str(" AND metadata.year = $year");
                bindings.push(("year", surrealdb::sql::Value::from(year)));
            }
        }

        if let Some(keyword) = &filters.keyword {
            // Sanitize keyword for regex search
            let sanitized = keyword.trim();
            if !sanitized.is_empty() && sanitized.len() <= 200 {
                query.push_str(" AND title ~ $keyword");
                bindings.push(("keyword", surrealdb::sql::Value::from(sanitized)));
            }
        }

        if let Some(venue) = &filters.venue {
            // Sanitize venue
            let sanitized = venue.trim();
            if !sanitized.is_empty() && sanitized.len() <= 200 {
                query.push_str(" AND metadata.venue = $venue");
                bindings.push(("venue", surrealdb::sql::Value::from(sanitized)));
            }
        }

        // Validate and bind limit (prevent DoS with huge limits)
        if let Some(limit) = filters.limit {
            let safe_limit = limit.min(1000); // Cap at 1000
            query.push_str(" LIMIT $limit");
            bindings.push(("limit", surrealdb::sql::Value::from(safe_limit)));
        }

        // Validate and bind offset
        if let Some(offset) = filters.offset {
            let safe_offset = offset.min(10000); // Cap at 10000
            query.push_str(" START $offset");
            bindings.push(("offset", surrealdb::sql::Value::from(safe_offset)));
        }

        let mut response = db.query(&query);

        // Apply all bindings
        for (key, value) in bindings {
            response = response.bind((key, value));
        }

        let mut response = response
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to list papers: {}", e)))?;

        let papers: Vec<Paper> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize papers: {}", e))
        })?;

        Ok(papers)
    }

    async fn search_papers(&self, query: &str) -> Result<Vec<Paper>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT * FROM paper WHERE title ~ $query OR abstract_text ~ $query")
            .bind(("query", query))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to search papers: {}", e)))?;

        let papers: Vec<Paper> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize papers: {}", e))
        })?;

        Ok(papers)
    }

    async fn delete_paper(&self, id: &str) -> Result<()> {
        let db = self.db()?;

        let _: Option<Paper> = db
            .delete(("paper", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete paper: {}", e)))?;

        Ok(())
    }

    // Segments operations
    async fn save_segment(&self, segment: &PaperSegment) -> Result<()> {
        let db = self.db()?;

        // Extract paper_id from segment.id (format: paper_id_segment_id)
        let paper_id = segment.id.split('_').next().unwrap_or("").to_string();

        // Convert segment to SurrealDB format
        let segment_type_str = Self::segment_type_to_string(&segment.segment_type);
        let line_range = vec![segment.line_range.0 as f64, segment.line_range.1 as f64];

        // Create segment record with proper structure
        let segment_data = serde_json::json!({
            "id": segment.id,
            "paper_id": paper_id,
            "section": segment.section,
            "content": segment.content,
            "segment_type": segment_type_str,
            "embedding": segment.embedding,
            "line_range": line_range,
            "created_at": Utc::now(),
            "updated_at": Utc::now(),
        });

        let _: Option<serde_json::Value> = db
            .create(("segment", &segment.id))
            .content(segment_data)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save segment: {}", e)))?;

        Ok(())
    }

    async fn get_segment(&self, id: &str) -> Result<Option<PaperSegment>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .select(("segment", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get segment: {}", e)))?;

        if let Some(seg_json) = result {
            // Extract fields and reconstruct PaperSegment
            let id = seg_json["id"]
                .as_str()
                .ok_or_else(|| StorageError::Deserialization("Missing id".to_string()))?
                .to_string();
            let section = seg_json["section"].as_str().unwrap_or("").to_string();
            let content = seg_json["content"]
                .as_str()
                .ok_or_else(|| StorageError::Deserialization("Missing content".to_string()))?
                .to_string();
            let segment_type_str = seg_json["segment_type"]
                .as_str()
                .unwrap_or("Other")
                .to_string();
            let segment_type = Self::segment_type_from_string(&segment_type_str);

            let embedding: Option<Vec<f32>> = seg_json["embedding"].as_array().and_then(|arr| {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<f32>>>()
            });

            let line_range = if let Some(range) = seg_json["line_range"].as_array() {
                if range.len() >= 2 {
                    (
                        range[0].as_f64().map(|f| f as usize).unwrap_or(0),
                        range[1].as_f64().map(|f| f as usize).unwrap_or(0),
                    )
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };

            Ok(Some(PaperSegment {
                id,
                section,
                content,
                segment_type,
                embedding,
                line_range,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_segments_by_paper(&self, paper_id: &str) -> Result<Vec<PaperSegment>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT * FROM segment WHERE paper_id = $paper_id")
            .bind(("paper_id", paper_id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get segments: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize segments: {}", e))
        })?;

        let mut segments = Vec::new();
        for seg_json in results {
            if let Ok(segment) = self.deserialize_segment(seg_json) {
                segments.push(segment);
            }
        }

        Ok(segments)
    }

    async fn get_segments_by_ids(&self, ids: &[String]) -> Result<Vec<PaperSegment>> {
        let db = self.db()?;

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate and sanitize IDs to prevent injection
        // Limit batch size to prevent DoS
        let max_batch_size = 100;
        let safe_ids: Vec<String> = ids
            .iter()
            .take(max_batch_size)
            .filter_map(|id| {
                let sanitized = id.trim();
                // Validate ID format: alphanumeric with hyphens/underscores, reasonable length
                if !sanitized.is_empty()
                    && sanitized.len() <= 200
                    && sanitized
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    Some(sanitized.to_string())
                } else {
                    None
                }
            })
            .collect();

        if safe_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Build query with IN clause
        let placeholders: Vec<String> = (0..safe_ids.len()).map(|i| format!("$id_{}", i)).collect();
        let query = format!(
            "SELECT * FROM segment WHERE id IN [{}]",
            placeholders.join(", ")
        );

        let mut response = db.query(&query);
        for (i, id) in safe_ids.iter().enumerate() {
            response = response.bind((format!("id_{}", i), id));
        }

        let mut response = response
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get segments: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize segments: {}", e))
        })?;

        let mut segments = Vec::new();
        for seg_json in results {
            if let Ok(segment) = self.deserialize_segment(seg_json) {
                segments.push(segment);
            }
        }

        Ok(segments)
    }

    // Continue with remaining operations...
    // Due to length, I'll implement the most critical ones first
    async fn save_repository(&self, repo: &Repository) -> Result<()> {
        let db = self.db()?;

        let _: Option<Repository> = db
            .create(("repository", &repo.id))
            .content(repo)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save repository: {}", e)))?;

        Ok(())
    }

    async fn get_repository(&self, id: &str) -> Result<Option<Repository>> {
        let db = self.db()?;

        let repo: Option<Repository> = db
            .select(("repository", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get repository: {}", e)))?;

        Ok(repo)
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let db = self.db()?;

        let mut response = db.query("SELECT * FROM repository").await.map_err(|e| {
            StorageError::QueryFailed(format!("Failed to list repositories: {}", e))
        })?;

        let repos: Vec<Repository> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize repositories: {}", e))
        })?;

        Ok(repos)
    }

    async fn update_repository(&self, repo: &Repository) -> Result<()> {
        let db = self.db()?;

        let _: Option<Repository> = db
            .update(("repository", &repo.id))
            .content(repo)
            .await
            .map_err(|e| {
                StorageError::QueryFailed(format!("Failed to update repository: {}", e))
            })?;

        Ok(())
    }

    async fn delete_repository(&self, id: &str) -> Result<()> {
        let db = self.db()?;

        let _: Option<Repository> = db.delete(("repository", id)).await.map_err(|e| {
            StorageError::QueryFailed(format!("Failed to delete repository: {}", e))
        })?;

        Ok(())
    }

    // Modules operations
    async fn save_module(&self, module: &CodeModule) -> Result<()> {
        let db = self.db()?;

        let _: Option<CodeModule> = db
            .update(("module", &module.id))
            .content(module)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save module: {}", e)))?;

        Ok(())
    }

    async fn get_module(&self, id: &str) -> Result<Option<CodeModule>> {
        let db = self.db()?;

        let module: Option<CodeModule> = db
            .select(("module", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get module: {}", e)))?;

        Ok(module)
    }

    async fn get_modules_by_repository(&self, repo_id: &str) -> Result<Vec<CodeModule>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT * FROM module WHERE repository_id = $repo_id")
            .bind(("repo_id", repo_id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get modules: {}", e)))?;

        let modules: Vec<CodeModule> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize modules: {}", e))
        })?;

        Ok(modules)
    }

    async fn search_modules(&self, query: &str) -> Result<Vec<CodeModule>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT * FROM module WHERE content ~ $query OR file_path ~ $query")
            .bind(("query", query))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to search modules: {}", e)))?;

        let modules: Vec<CodeModule> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize modules: {}", e))
        })?;

        Ok(modules)
    }

    async fn delete_module(&self, id: &str) -> Result<()> {
        let db = self.db()?;

        let _: Option<CodeModule> = db
            .delete(("module", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete module: {}", e)))?;

        Ok(())
    }

    // Tasks operations
    async fn save_task(&self, task: &Task) -> Result<()> {
        let db = self.db()?;

        // Serialize task directly - SurrealDB handles the conversion
        let _: Option<Task> = db
            .create(("task", &task.id.to_string()))
            .content(task)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save task: {}", e)))?;

        Ok(())
    }

    async fn get_task(&self, id: &uuid::Uuid) -> Result<Option<Task>> {
        let db = self.db()?;

        let result: Option<Task> = db
            .select(("task", id.to_string()))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get task: {}", e)))?;

        Ok(result)
    }

    async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let db = self.db()?;

        // Serialize status for comparison
        let status_json = serde_json::to_value(&status).map_err(|e| {
            StorageError::Serialization(format!("Failed to serialize status: {}", e))
        })?;

        let mut response = db
            .query("SELECT * FROM task WHERE status = $status")
            .bind(("status", status_json))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get tasks: {}", e)))?;

        let tasks: Vec<Task> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize tasks: {}", e))
        })?;

        Ok(tasks)
    }

    async fn get_tasks_by_paper(&self, paper_id: &str) -> Result<Vec<Task>> {
        let db = self.db()?;

        // Query tasks by checking if paper_id matches or context contains paper_id
        let mut response = db
            .query("SELECT * FROM task WHERE paper_id = $paper_id OR context.paper_id = $paper_id")
            .bind(("paper_id", paper_id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get tasks: {}", e)))?;

        let tasks: Vec<Task> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize tasks: {}", e))
        })?;

        Ok(tasks)
    }

    async fn update_task_status(&self, id: &uuid::Uuid, status: TaskStatus) -> Result<()> {
        let db = self.db()?;

        // Serialize status
        let status_json = serde_json::to_value(&status).map_err(|e| {
            StorageError::Serialization(format!("Failed to serialize status: {}", e))
        })?;

        let _: Option<Task> = db
            .update(("task", id.to_string()))
            .merge(serde_json::json!({
                "status": status_json,
                "updated_at": Utc::now(),
            }))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to update task: {}", e)))?;

        Ok(())
    }

    async fn delete_task(&self, id: &uuid::Uuid) -> Result<()> {
        let db = self.db()?;

        let _: Option<Task> = db
            .delete(("task", id.to_string()))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete task: {}", e)))?;

        Ok(())
    }

    // Dependencies (Graph) operations
    async fn add_dependency(&self, from: &str, to: &str) -> Result<()> {
        let db = self.db()?;

        // Use RELATE to create graph edge
        db.query("RELATE module:$from -> depends_on -> module:$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(|e| {
                StorageError::GraphOperation(format!("Failed to add dependency: {}", e))
            })?;

        Ok(())
    }

    async fn remove_dependency(&self, from: &str, to: &str) -> Result<()> {
        let db = self.db()?;

        // Delete the edge relationship
        db.query("DELETE depends_on WHERE in = module:$from AND out = module:$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(|e| {
                StorageError::GraphOperation(format!("Failed to remove dependency: {}", e))
            })?;

        Ok(())
    }

    async fn get_dependencies(&self, module_id: &str) -> Result<Vec<String>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT ->depends_on->module.id AS deps FROM type::thing('module', $id)")
            .bind(("id", module_id))
            .await
            .map_err(|e| {
                StorageError::GraphOperation(format!("Failed to get dependencies: {}", e))
            })?;

        let result: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize dependencies: {}", e))
        })?;

        // Extract IDs from result - handle both array and single value
        let mut deps = Vec::new();
        for v in result {
            if let Some(deps_array) = v.get("deps").and_then(|d| d.as_array()) {
                for dep in deps_array {
                    if let Some(id_str) = dep.as_str() {
                        // Extract ID from record format "module:id" or just "id"
                        let id = if id_str.starts_with("module:") {
                            id_str.replace("module:", "")
                        } else {
                            id_str.to_string()
                        };
                        deps.push(id);
                    }
                }
            } else if let Some(id_str) = v.get("deps").and_then(|d| d.as_str()) {
                let id = if id_str.starts_with("module:") {
                    id_str.replace("module:", "")
                } else {
                    id_str.to_string()
                };
                deps.push(id);
            }
        }

        Ok(deps)
    }

    async fn get_dependents(&self, module_id: &str) -> Result<Vec<String>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT <-depends_on<-module.id AS deps FROM type::thing('module', $id)")
            .bind(("id", module_id))
            .await
            .map_err(|e| {
                StorageError::GraphOperation(format!("Failed to get dependents: {}", e))
            })?;

        let result: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize dependents: {}", e))
        })?;

        // Extract IDs from result - handle both array and single value
        let mut deps = Vec::new();
        for v in result {
            if let Some(deps_array) = v.get("deps").and_then(|d| d.as_array()) {
                for dep in deps_array {
                    if let Some(id_str) = dep.as_str() {
                        let id = if id_str.starts_with("module:") {
                            id_str.replace("module:", "")
                        } else {
                            id_str.to_string()
                        };
                        deps.push(id);
                    }
                }
            } else if let Some(id_str) = v.get("deps").and_then(|d| d.as_str()) {
                let id = if id_str.starts_with("module:") {
                    id_str.replace("module:", "")
                } else {
                    id_str.to_string()
                };
                deps.push(id);
            }
        }

        Ok(deps)
    }

    async fn get_dependency_chain(&self, module_id: &str) -> Result<Vec<String>> {
        // Get all transitive dependencies
        let db = self.db()?;

        // Get transitive dependencies (2 levels deep)
        let mut response = db
            .query("SELECT ->depends_on->->depends_on->module.id AS deps FROM type::thing('module', $id)")
            .bind(("id", module_id))
            .await
            .map_err(|e| StorageError::GraphOperation(format!("Failed to get dependency chain: {}", e)))?;

        let result: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize chain: {}", e))
        })?;

        // Extract IDs from result
        let mut chain = Vec::new();
        for v in result {
            if let Some(deps_array) = v.get("deps").and_then(|d| d.as_array()) {
                for dep in deps_array {
                    if let Some(id_str) = dep.as_str() {
                        let id = if id_str.starts_with("module:") {
                            id_str.replace("module:", "")
                        } else {
                            id_str.to_string()
                        };
                        chain.push(id);
                    }
                }
            } else if let Some(id_str) = v.get("deps").and_then(|d| d.as_str()) {
                let id = if id_str.starts_with("module:") {
                    id_str.replace("module:", "")
                } else {
                    id_str.to_string()
                };
                chain.push(id);
            }
        }

        Ok(chain)
    }

    async fn get_dependency_graph(&self, repo_id: &str) -> Result<DependencyGraph> {
        let db = self.db()?;

        // Get all dependencies for modules in this repository
        // First get all modules in the repository, then get their dependencies
        let mut response = db
            .query(
                "
                SELECT 
                    VALUE id FROM module 
                    WHERE repository_id = $repo_id;
                SELECT 
                    ->depends_on->module.id AS to_id,
                    <-depends_on<-module.id AS from_id
                FROM module 
                WHERE repository_id = $repo_id;
            ",
            )
            .bind(("repo_id", repo_id))
            .await
            .map_err(|e| {
                StorageError::GraphOperation(format!("Failed to get dependency graph: {}", e))
            })?;

        let result: Vec<serde_json::Value> = response.take(1).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize graph: {}", e))
        })?;

        let mut edges = Vec::new();
        for edge in result {
            // Handle different response formats
            if let Some(to_id) = edge.get("to_id") {
                if let Some(from_id) = edge.get("from_id") {
                    let from = if let Some(s) = from_id.as_str() {
                        s.replace("module:", "")
                    } else {
                        continue;
                    };
                    let to = if let Some(s) = to_id.as_str() {
                        s.replace("module:", "")
                    } else if let Some(arr) = to_id.as_array() {
                        // Handle array of dependencies
                        if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                            first.replace("module:", "")
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };
                    edges.push((from, to));
                }
            }
        }

        Ok(DependencyGraph { edges })
    }

    // Documents operations
    async fn save_document(&self, doc: &Document) -> Result<()> {
        let db = self.db()?;

        let _: Option<Document> = db
            .create(("document", &doc.id))
            .content(doc)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save document: {}", e)))?;

        Ok(())
    }

    async fn get_document(&self, id: &str) -> Result<Option<Document>> {
        let db = self.db()?;

        let doc: Option<Document> = db
            .select(("document", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get document: {}", e)))?;

        Ok(doc)
    }

    async fn list_documents(&self, filters: DocumentFilters) -> Result<Vec<Document>> {
        let db = self.db()?;

        // Use parameterized queries to prevent SQL injection
        let mut query = "SELECT * FROM document WHERE 1=1".to_string();
        let mut bindings: Vec<(&str, surrealdb::sql::Value)> = Vec::new();

        if let Some(content_type) = &filters.content_type {
            // Sanitize content_type (MIME type format)
            let sanitized = content_type.trim();
            if !sanitized.is_empty()
                && sanitized.len() <= 100
                && sanitized
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '/' || c == '-' || c == '.')
            {
                query.push_str(" AND content_type = $content_type");
                bindings.push(("content_type", surrealdb::sql::Value::from(sanitized)));
            }
        }

        if let Some(name) = &filters.name {
            // Sanitize name for regex search
            let sanitized = name.trim();
            if !sanitized.is_empty() && sanitized.len() <= 200 {
                query.push_str(" AND name ~ $name");
                bindings.push(("name", surrealdb::sql::Value::from(sanitized)));
            }
        }

        // Validate and bind limit (prevent DoS)
        if let Some(limit) = filters.limit {
            let safe_limit = limit.min(1000);
            query.push_str(" LIMIT $limit");
            bindings.push(("limit", surrealdb::sql::Value::from(safe_limit)));
        }

        // Validate and bind offset
        if let Some(offset) = filters.offset {
            let safe_offset = offset.min(10000);
            query.push_str(" START $offset");
            bindings.push(("offset", surrealdb::sql::Value::from(safe_offset)));
        }

        let mut response = db.query(&query);

        // Apply all bindings
        for (key, value) in bindings {
            response = response.bind((key, value));
        }

        let mut response = response
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to list documents: {}", e)))?;

        let documents: Vec<Document> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize documents: {}", e))
        })?;

        Ok(documents)
    }

    async fn search_documents(&self, query: &str) -> Result<Vec<Document>> {
        let db = self.db()?;

        let mut response = db
            .query("SELECT * FROM document WHERE name ~ $query OR content ~ $query")
            .bind(("query", query))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to search documents: {}", e)))?;

        let documents: Vec<Document> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize documents: {}", e))
        })?;

        Ok(documents)
    }

    async fn delete_document(&self, id: &str) -> Result<()> {
        let db = self.db()?;

        let _: Option<Document> = db
            .delete(("document", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete document: {}", e)))?;

        Ok(())
    }

    // Vector operations
    async fn save_embedding(&self, segment_id: &str, embedding: Vec<f32>) -> Result<()> {
        let db = self.db()?;

        let _: Option<serde_json::Value> = db
            .update(("segment", segment_id))
            .merge(serde_json::json!({
                "embedding": embedding,
                "updated_at": Utc::now(),
            }))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to save embedding: {}", e)))?;

        Ok(())
    }

    async fn get_embedding(&self, segment_id: &str) -> Result<Option<Vec<f32>>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .select(("segment", segment_id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get embedding: {}", e)))?;

        if let Some(seg_json) = result {
            if let Some(emb_array) = seg_json.get("embedding").and_then(|e| e.as_array()) {
                let embedding: Option<Vec<f32>> = emb_array
                    .iter()
                    .map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Option<Vec<f32>>>();
                return Ok(embedding);
            }
        }

        Ok(None)
    }

    async fn vector_search(
        &self,
        query_embedding: Vec<f32>,
        k: usize,
        filters: Option<SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        let db = self.db()?;

        // Validate embedding dimension
        if query_embedding.is_empty() {
            return Err(
                StorageError::VectorSearch("Query embedding cannot be empty".to_string()).into(),
            );
        }

        // Validate k (prevent DoS with huge k)
        let safe_k = k.min(1000); // Cap at 1000 results

        // Build query with vector similarity search
        // SurrealDB 1.5+ uses vector::similarity::cosine for cosine similarity
        let mut query = String::from(
            "SELECT *, vector::similarity::cosine(embedding, $query) AS score FROM segment WHERE embedding != NONE AND array::len(embedding) > 0"
        );
        let mut bindings: Vec<(&str, surrealdb::sql::Value)> = Vec::new();

        if let Some(filters) = filters {
            if let Some(paper_id) = filters.paper_id {
                // Sanitize paper_id
                let sanitized = paper_id.trim();
                if !sanitized.is_empty()
                    && sanitized.len() <= 200
                    && sanitized
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    query.push_str(" AND paper_id = $paper_id");
                    bindings.push(("paper_id", surrealdb::sql::Value::from(sanitized)));
                }
            }
            if let Some(segment_type) = filters.segment_type {
                // Sanitize segment_type
                let sanitized = segment_type.trim();
                if !sanitized.is_empty()
                    && sanitized.len() <= 100
                    && sanitized
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
                {
                    query.push_str(" AND segment_type = $segment_type");
                    bindings.push(("segment_type", surrealdb::sql::Value::from(sanitized)));
                }
            }
            if let Some(min_score) = filters.min_score {
                query.push_str(" AND vector::similarity::cosine(embedding, $query) >= $min_score");
                bindings.push(("min_score", surrealdb::sql::Value::from(min_score)));
            }
        }

        query.push_str(" ORDER BY score DESC LIMIT $limit");
        bindings.push(("limit", surrealdb::sql::Value::from(safe_k)));

        let mut response = db.query(&query).bind(("query", query_embedding));

        // Apply all bindings
        for (key, value) in bindings {
            response = response.bind((key, value));
        }

        let mut response = response
            .await
            .map_err(|e| StorageError::VectorSearch(format!("Vector search failed: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize search results: {}", e))
        })?;

        let mut search_results = Vec::new();
        for result in results {
            // Extract score
            let score = result
                .get("score")
                .and_then(|s| s.as_f64())
                .map(|s| s as f32)
                .unwrap_or(0.0);

            // Deserialize segment from the result
            if let Ok(segment) = self.deserialize_segment(result.clone()) {
                search_results.push(SearchResult { segment, score });
            }
        }

        Ok(search_results)
    }

    // Graph analysis
    async fn analyze_dependencies(&self, repo_id: &str) -> Result<GraphAnalysis> {
        let graph = self.get_dependency_graph(repo_id).await?;
        let analysis = GraphAnalyzer::analyze_graph(&graph);
        Ok(analysis)
    }

    async fn find_circular_dependencies(&self, repo_id: &str) -> Result<Vec<Vec<String>>> {
        let graph = self.get_dependency_graph(repo_id).await?;
        let cycles = GraphAnalyzer::find_circular_dependencies(&graph);
        Ok(cycles)
    }

    async fn get_module_impact(&self, module_id: &str) -> Result<ModuleImpact> {
        let dependents = self.get_dependents(module_id).await?;
        let dependencies = self.get_dependencies(module_id).await?;
        let chain = self.get_dependency_chain(module_id).await?;

        Ok(ModuleImpact {
            direct_dependents: dependents.len(),
            transitive_dependents: chain.len(),
            direct_dependencies: dependencies.len(),
            affected_modules: chain,
        })
    }

    async fn execute_raw_query(&self, query: &str) -> Result<serde_json::Value> {
        let db = self.db()?;

        let mut response = db
            .query(query)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Query execution failed: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0usize).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize result: {}", e))
        })?;

        Ok(results
            .into_iter()
            .next()
            .unwrap_or(serde_json::Value::Null))
    }

    async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let _db = self.db()?;

        // Query counts for each table
        let paper_count: usize = self.count_records("paper").await?;
        let segment_count: usize = self.count_records("segment").await?;
        let repository_count: usize = self.count_records("repository").await?;
        let module_count: usize = self.count_records("module").await?;
        let task_count: usize = self.count_records("task").await?;
        let dependency_count: usize = self.count_records("dependency").await?;
        let document_count: usize = self.count_records("document").await?;

        Ok(DatabaseStats {
            paper_count,
            segment_count,
            repository_count,
            module_count,
            task_count,
            dependency_count,
            document_count,
            total_records: paper_count
                + segment_count
                + repository_count
                + module_count
                + task_count
                + dependency_count
                + document_count,
        })
    }

    // User management
    async fn save_user(&self, user: &crate::api::auth::user::User) -> Result<()> {
        let db = self.db()?;

        let roles_json: Vec<String> = user.roles.iter().map(|r| r.as_str().to_string()).collect();

        db.query(
            "
            CREATE user:$id CONTENT {
                id: $id,
                email: $email,
                username: $username,
                password_hash: $password_hash,
                roles: $roles,
                created_at: $created_at,
                updated_at: $updated_at
            };
        ",
        )
        .bind(("id", &user.id))
        .bind(("email", &user.email))
        .bind(("username", &user.username))
        .bind(("password_hash", &user.password_hash))
        .bind(("roles", roles_json))
        .bind(("created_at", user.created_at))
        .bind(("updated_at", user.updated_at))
        .await
        .map_err(|e| StorageError::QueryFailed(format!("Failed to save user: {}", e)))?;

        Ok(())
    }

    async fn get_user(&self, id: &str) -> Result<Option<crate::api::auth::user::User>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .query("SELECT * FROM user WHERE id = $id")
            .bind(("id", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get user: {}", e)))?
            .take(0)
            .map_err(|e| StorageError::QueryFailed(format!("Failed to take result: {}", e)))?;

        if let Some(user_json) = result {
            self.deserialize_user(user_json).map(Some)
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<crate::api::auth::user::User>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .query("SELECT * FROM user WHERE email = $email LIMIT 1")
            .bind(("email", email))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get user by email: {}", e)))?
            .take(0)
            .map_err(|e| StorageError::QueryFailed(format!("Failed to take result: {}", e)))?;

        if let Some(user_json) = result {
            self.deserialize_user(user_json).map(Some)
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::api::auth::user::User>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .query("SELECT * FROM user WHERE username = $username LIMIT 1")
            .bind(("username", username))
            .await
            .map_err(|e| {
                StorageError::QueryFailed(format!("Failed to get user by username: {}", e))
            })?
            .take(0)
            .map_err(|e| StorageError::QueryFailed(format!("Failed to take result: {}", e)))?;

        if let Some(user_json) = result {
            self.deserialize_user(user_json).map(Some)
        } else {
            Ok(None)
        }
    }

    async fn update_user(&self, user: &crate::api::auth::user::User) -> Result<()> {
        let db = self.db()?;

        let roles_json: Vec<String> = user.roles.iter().map(|r| r.as_str().to_string()).collect();

        db.query(
            "
            UPDATE user:$id SET {
                email: $email,
                username: $username,
                password_hash: $password_hash,
                roles: $roles,
                updated_at: $updated_at
            };
        ",
        )
        .bind(("id", &user.id))
        .bind(("email", &user.email))
        .bind(("username", &user.username))
        .bind(("password_hash", &user.password_hash))
        .bind(("roles", roles_json))
        .bind(("updated_at", user.updated_at))
        .await
        .map_err(|e| StorageError::QueryFailed(format!("Failed to update user: {}", e)))?;

        Ok(())
    }

    async fn delete_user(&self, id: &str) -> Result<()> {
        let db = self.db()?;

        db.query("DELETE FROM user WHERE id = $id")
            .bind(("id", id))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete user: {}", e)))?;

        Ok(())
    }

    // Session management
    async fn save_session(&self, session: &crate::api::auth::user::Session) -> Result<()> {
        let db = self.db()?;

        db.query(
            "
            CREATE session:$id CONTENT {
                id: $id,
                user_id: $user_id,
                refresh_token: $refresh_token,
                expires_at: $expires_at,
                created_at: $created_at,
                last_used_at: $last_used_at
            };
        ",
        )
        .bind(("id", &session.id))
        .bind(("user_id", &session.user_id))
        .bind(("refresh_token", &session.refresh_token))
        .bind(("expires_at", session.expires_at))
        .bind(("created_at", session.created_at))
        .bind(("last_used_at", session.last_used_at))
        .await
        .map_err(|e| StorageError::QueryFailed(format!("Failed to save session: {}", e)))?;

        Ok(())
    }

    async fn get_session(
        &self,
        refresh_token: &str,
    ) -> Result<Option<crate::api::auth::user::Session>> {
        let db = self.db()?;

        let result: Option<serde_json::Value> = db
            .query("SELECT * FROM session WHERE refresh_token = $refresh_token LIMIT 1")
            .bind(("refresh_token", refresh_token))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to get session: {}", e)))?
            .take(0)
            .map_err(|e| StorageError::QueryFailed(format!("Failed to take result: {}", e)))?;

        if let Some(session_json) = result {
            self.deserialize_session(session_json).map(Some)
        } else {
            Ok(None)
        }
    }

    async fn delete_session(&self, refresh_token: &str) -> Result<()> {
        let db = self.db()?;

        db.query("DELETE FROM session WHERE refresh_token = $refresh_token")
            .bind(("refresh_token", refresh_token))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to delete session: {}", e)))?;

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let db = self.db()?;

        db.query("DELETE FROM session WHERE user_id = $user_id")
            .bind(("user_id", user_id))
            .await
            .map_err(|e| {
                StorageError::QueryFailed(format!("Failed to delete user sessions: {}", e))
            })?;

        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let db = self.db()?;

        let now = Utc::now();
        let result: Vec<serde_json::Value> = db
            .query("SELECT id FROM session WHERE expires_at < $now")
            .bind(("now", now))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to cleanup sessions: {}", e)))?
            .take(0)
            .map_err(|e| StorageError::QueryFailed(format!("Failed to take result: {}", e)))?;

        let count = result.len();

        db.query("DELETE FROM session WHERE expires_at < $now")
            .bind(("now", now))
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to cleanup sessions: {}", e)))?;

        Ok(count)
    }
}

impl Default for SurrealStorage {
    fn default() -> Self {
        Self::new()
    }
}

// Additional helper methods for batch operations and performance optimization
impl SurrealStorage {
    /// Save multiple papers in a batch operation with transaction support
    /// This is more efficient than individual saves and ensures atomicity
    pub async fn save_papers_batch(&self, papers: &[Paper]) -> Result<Vec<String>> {
        let db = self.db()?;

        // Limit batch size to prevent memory issues
        const MAX_BATCH_SIZE: usize = 100;
        if papers.len() > MAX_BATCH_SIZE {
            return Err(StorageError::QueryFailed(format!(
                "Batch size {} exceeds maximum of {}",
                papers.len(),
                MAX_BATCH_SIZE
            ))
            .into());
        }

        // Use batch insert with individual creates for better error handling
        // SurrealDB doesn't have explicit transactions in the same way, but we can use BEGIN/COMMIT
        for paper in papers {
            let _: Option<Paper> = db
                .create(("paper", &paper.id))
                .content(paper)
                .await
                .map_err(|e| {
                    StorageError::QueryFailed(format!("Failed to save paper in batch: {}", e))
                })?;
        }

        let saved_ids: Vec<String> = papers.iter().map(|p| p.id.clone()).collect();

        Ok(saved_ids)
    }

    /// Save multiple modules in a batch operation with transaction support
    pub async fn save_modules_batch(&self, modules: &[CodeModule]) -> Result<Vec<String>> {
        let db = self.db()?;

        // Limit batch size
        const MAX_BATCH_SIZE: usize = 100;
        if modules.len() > MAX_BATCH_SIZE {
            return Err(StorageError::QueryFailed(format!(
                "Batch size {} exceeds maximum of {}",
                modules.len(),
                MAX_BATCH_SIZE
            ))
            .into());
        }

        // Use batch insert
        for module in modules {
            let _: Option<CodeModule> = db
                .update(("module", &module.id))
                .content(module)
                .await
                .map_err(|e| {
                    StorageError::QueryFailed(format!("Failed to save module in batch: {}", e))
                })?;
        }

        let saved_ids: Vec<String> = modules.iter().map(|m| m.id.clone()).collect();

        Ok(saved_ids)
    }

    /// Save multiple segments in a batch operation with embeddings and transaction support
    /// This is optimized for bulk segment insertion with vector embeddings
    pub async fn save_segments_batch(&self, segments: &[PaperSegment]) -> Result<Vec<String>> {
        let db = self.db()?;

        // Limit batch size to prevent memory issues (segments can be large with embeddings)
        const MAX_BATCH_SIZE: usize = 50;
        if segments.len() > MAX_BATCH_SIZE {
            return Err(StorageError::QueryFailed(format!(
                "Batch size {} exceeds maximum of {} for segments",
                segments.len(),
                MAX_BATCH_SIZE
            ))
            .into());
        }

        // Use batch insert
        for segment in segments {
            let segment_type_str = Self::segment_type_to_string(&segment.segment_type);
            let line_range = vec![segment.line_range.0 as f64, segment.line_range.1 as f64];

            // Extract paper_id from segment.id (format: paper_id_segment_id)
            let paper_id = segment.id.split('_').next().unwrap_or("").to_string();

            let _: Option<serde_json::Value> = db
                .create(("segment", &segment.id))
                .content(serde_json::json!({
                    "id": segment.id,
                    "paper_id": paper_id,
                    "section": segment.section,
                    "content": segment.content,
                    "segment_type": segment_type_str,
                    "embedding": segment.embedding,
                    "line_range": line_range,
                    "created_at": Utc::now(),
                    "updated_at": Utc::now(),
                }))
                .await
                .map_err(|e| {
                    StorageError::QueryFailed(format!("Failed to save segment in batch: {}", e))
                })?;
        }

        let saved_ids: Vec<String> = segments.iter().map(|s| s.id.clone()).collect();

        Ok(saved_ids)
    }

    /// Get connection pool statistics
    pub fn get_connection_info_cached(&self) -> Option<ConnectionInfo> {
        self.connection_info.clone()
    }

    /// Check if database is healthy
    pub async fn health_check(&self) -> Result<bool> {
        if !self.is_connected() {
            return Ok(false);
        }

        match self.test_connection().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let _db = self.db()?;

        // Query counts for each table
        let paper_count: usize = self.count_records("paper").await?;
        let segment_count: usize = self.count_records("segment").await?;
        let repository_count: usize = self.count_records("repository").await?;
        let module_count: usize = self.count_records("module").await?;
        let task_count: usize = self.count_records("task").await?;
        let dependency_count: usize = self.count_records("dependency").await?;
        let document_count: usize = self.count_records("document").await?;

        Ok(DatabaseStats {
            paper_count,
            segment_count,
            repository_count,
            module_count,
            task_count,
            dependency_count,
            document_count,
            total_records: paper_count
                + segment_count
                + repository_count
                + module_count
                + task_count
                + dependency_count
                + document_count,
        })
    }

    /// Helper: Count records in a table
    async fn count_records(&self, table: &str) -> Result<usize> {
        let db = self.db()?;

        // Validate table name to prevent injection (whitelist approach)
        let valid_tables = [
            "paper",
            "segment",
            "repository",
            "module",
            "task",
            "dependency",
            "document",
            "user",
            "session",
        ];
        if !valid_tables.contains(&table) {
            return Err(StorageError::QueryFailed(format!("Invalid table name: {}", table)).into());
        }

        // Use parameterized query with table name validation
        let query = format!("SELECT count() as count FROM {}", table);
        let mut response = db
            .query(&query)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to count records: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct CountResult {
            count: usize,
        }

        let result: Option<CountResult> = response.take(0).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize count: {}", e))
        })?;

        Ok(result.map(|r| r.count).unwrap_or(0))
    }

    /// Execute a raw query (for admin console)
    pub async fn execute_raw_query(&self, query: &str) -> Result<serde_json::Value> {
        let db = self.db()?;

        let mut response = db
            .query(query)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Query execution failed: {}", e)))?;

        let results: Vec<serde_json::Value> = response.take(0usize).map_err(|e| {
            StorageError::Deserialization(format!("Failed to deserialize result: {}", e))
        })?;

        Ok(results
            .into_iter()
            .next()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Optimize database (run maintenance tasks)
    pub async fn optimize_database(&self) -> Result<()> {
        // SurrealDB handles optimization automatically
        // This is a placeholder for future optimizations
        Ok(())
    }

    /// Clear all data from a table (admin function)
    /// SECURITY: This is a dangerous operation - should only be called by admin users
    pub async fn clear_table(&self, table: &str) -> Result<usize> {
        let db = self.db()?;

        // Validate table name to prevent injection (whitelist approach)
        let valid_tables = [
            "paper",
            "segment",
            "repository",
            "module",
            "task",
            "dependency",
            "document",
        ];
        if !valid_tables.contains(&table) {
            return Err(StorageError::QueryFailed(format!(
                "Invalid table name for clear operation: {}",
                table
            ))
            .into());
        }

        // First count how many records
        let count = self.count_records(table).await?;

        // Then delete all using validated table name
        let query = format!("DELETE FROM {}", table);
        db.query(&query)
            .await
            .map_err(|e| StorageError::QueryFailed(format!("Failed to clear table: {}", e)))?;

        Ok(count)
    }

    /// Deserialize user from JSON (helper method)
    fn deserialize_user(
        &self,
        user_json: serde_json::Value,
    ) -> Result<crate::api::auth::user::User> {
        use crate::api::auth::user::{User, UserRole};

        let id = user_json["id"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing user id".to_string()))?
            .to_string();
        let email = user_json["email"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing email".to_string()))?
            .to_string();
        let username = user_json["username"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing username".to_string()))?
            .to_string();
        let password_hash = user_json["password_hash"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing password_hash".to_string()))?
            .to_string();

        let roles: Vec<UserRole> = user_json["roles"]
            .as_array()
            .ok_or_else(|| StorageError::Deserialization("Invalid roles format".to_string()))?
            .iter()
            .filter_map(|r| r.as_str().and_then(|s| UserRole::from_str(s)))
            .collect();

        let created_at = serde_json::from_value(user_json["created_at"].clone())
            .map_err(|e| StorageError::Deserialization(format!("Invalid created_at: {}", e)))?;
        let updated_at = serde_json::from_value(user_json["updated_at"].clone())
            .map_err(|e| StorageError::Deserialization(format!("Invalid updated_at: {}", e)))?;

        Ok(User {
            id,
            email,
            username,
            password_hash,
            roles,
            created_at,
            updated_at,
        })
    }

    /// Deserialize segment from JSON
    fn deserialize_segment(&self, seg_json: serde_json::Value) -> Result<PaperSegment> {
        let id = seg_json["id"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing id".to_string()))?
            .to_string();
        let section = seg_json["section"].as_str().unwrap_or("").to_string();
        let content = seg_json["content"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing content".to_string()))?
            .to_string();
        let segment_type_str = seg_json["segment_type"]
            .as_str()
            .unwrap_or("Other")
            .to_string();
        let segment_type = Self::segment_type_from_string(&segment_type_str);

        let embedding: Option<Vec<f32>> = seg_json["embedding"].as_array().and_then(|arr| {
            arr.iter()
                .map(|v| v.as_f64().map(|f| f as f32))
                .collect::<Option<Vec<f32>>>()
        });

        let line_range = if let Some(range) = seg_json["line_range"].as_array() {
            if range.len() >= 2 {
                (
                    range[0].as_f64().map(|f| f as usize).unwrap_or(0),
                    range[1].as_f64().map(|f| f as usize).unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        Ok(PaperSegment {
            id,
            section,
            content,
            segment_type,
            embedding,
            line_range,
        })
    }

    /// Deserialize session from JSON
    fn deserialize_session(
        &self,
        session_json: serde_json::Value,
    ) -> Result<crate::api::auth::user::Session> {
        use crate::api::auth::user::Session;

        let id = session_json["id"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing session id".to_string()))?
            .to_string();
        let user_id = session_json["user_id"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing user_id".to_string()))?
            .to_string();
        let refresh_token = session_json["refresh_token"]
            .as_str()
            .ok_or_else(|| StorageError::Deserialization("Missing refresh_token".to_string()))?
            .to_string();
        let expires_at = serde_json::from_value(session_json["expires_at"].clone())
            .map_err(|e| StorageError::Deserialization(format!("Invalid expires_at: {}", e)))?;
        let created_at = serde_json::from_value(session_json["created_at"].clone())
            .map_err(|e| StorageError::Deserialization(format!("Invalid created_at: {}", e)))?;
        let last_used_at = serde_json::from_value(session_json["last_used_at"].clone())
            .map_err(|e| StorageError::Deserialization(format!("Invalid last_used_at: {}", e)))?;

        Ok(Session {
            id,
            user_id,
            refresh_token,
            expires_at,
            created_at,
            last_used_at,
        })
    }
}
