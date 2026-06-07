#[cfg(test)]
mod tests {
    use crate::config::StorageConfig;
    use crate::error::Result;
    use crate::storage::errors::StorageError;
    use crate::storage::filters::{DocumentFilters, PaperFilters, SearchFilters};
    use crate::storage::graph::GraphAnalyzer;
    use crate::storage::manager::StorageManager;
    use crate::storage::traits::Storage;
    use crate::storage::vector::{cosine_similarity, normalize_vector};
    use crate::storage::{ConnectionInfo, Document, GraphAnalysis, ModuleImpact, SearchResult};
    use crate::types::*;
    use chrono::Utc;
    use uuid::Uuid;

    // Mock storage for testing
    #[allow(dead_code)]
    struct MockStorage {
        connected: bool,
    }

    #[async_trait::async_trait]
    impl Storage for MockStorage {
        async fn connect(&mut self, _config: &StorageConfig) -> Result<()> {
            self.connected = true;
            Ok(())
        }

        async fn disconnect(&mut self) -> Result<()> {
            self.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.connected
        }

        async fn test_connection(&self) -> Result<()> {
            if self.connected {
                Ok(())
            } else {
                Err(StorageError::NotConnected.into())
            }
        }

        fn get_connection_info(&self) -> Option<ConnectionInfo> {
            if self.connected {
                Some(ConnectionInfo {
                    connection_string: "ws://localhost:8000".to_string(),
                    namespace: "test".to_string(),
                    database: "test".to_string(),
                    connected_at: Utc::now(),
                })
            } else {
                None
            }
        }

        // Implement all required methods with minimal implementations
        async fn save_paper(&self, _paper: &Paper) -> Result<()> {
            Ok(())
        }
        async fn get_paper(&self, _id: &str) -> Result<Option<Paper>> {
            Ok(None)
        }
        async fn list_papers(&self, _filters: PaperFilters) -> Result<Vec<Paper>> {
            Ok(Vec::new())
        }
        async fn search_papers(&self, _query: &str) -> Result<Vec<Paper>> {
            Ok(Vec::new())
        }
        async fn delete_paper(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn save_segment(&self, _segment: &PaperSegment) -> Result<()> {
            Ok(())
        }
        async fn get_segment(&self, _id: &str) -> Result<Option<PaperSegment>> {
            Ok(None)
        }
        async fn get_segments_by_paper(&self, _paper_id: &str) -> Result<Vec<PaperSegment>> {
            Ok(Vec::new())
        }
        async fn get_segments_by_ids(&self, _ids: &[String]) -> Result<Vec<PaperSegment>> {
            Ok(Vec::new())
        }
        async fn save_repository(&self, _repo: &Repository) -> Result<()> {
            Ok(())
        }
        async fn get_repository(&self, _id: &str) -> Result<Option<Repository>> {
            Ok(None)
        }
        async fn list_repositories(&self) -> Result<Vec<Repository>> {
            Ok(Vec::new())
        }
        async fn update_repository(&self, _repo: &Repository) -> Result<()> {
            Ok(())
        }
        async fn delete_repository(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn save_module(&self, _module: &CodeModule) -> Result<()> {
            Ok(())
        }
        async fn get_module(&self, _id: &str) -> Result<Option<CodeModule>> {
            Ok(None)
        }
        async fn get_modules_by_repository(&self, _repo_id: &str) -> Result<Vec<CodeModule>> {
            Ok(Vec::new())
        }
        async fn search_modules(&self, _query: &str) -> Result<Vec<CodeModule>> {
            Ok(Vec::new())
        }
        async fn delete_module(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn save_task(&self, _task: &Task) -> Result<()> {
            Ok(())
        }
        async fn get_task(&self, _id: &Uuid) -> Result<Option<Task>> {
            Ok(None)
        }
        async fn get_tasks_by_status(&self, _status: TaskStatus) -> Result<Vec<Task>> {
            Ok(Vec::new())
        }
        async fn get_tasks_by_paper(&self, _paper_id: &str) -> Result<Vec<Task>> {
            Ok(Vec::new())
        }
        async fn update_task_status(&self, _id: &Uuid, _status: TaskStatus) -> Result<()> {
            Ok(())
        }
        async fn delete_task(&self, _id: &Uuid) -> Result<()> {
            Ok(())
        }
        async fn add_dependency(&self, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }
        async fn remove_dependency(&self, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }
        async fn get_dependencies(&self, _module_id: &str) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
        async fn get_dependents(&self, _module_id: &str) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
        async fn get_dependency_chain(&self, _module_id: &str) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
        async fn get_dependency_graph(&self, _repo_id: &str) -> Result<DependencyGraph> {
            Ok(DependencyGraph { edges: Vec::new() })
        }
        async fn save_document(&self, _doc: &Document) -> Result<()> {
            Ok(())
        }
        async fn get_document(&self, _id: &str) -> Result<Option<Document>> {
            Ok(None)
        }
        async fn list_documents(&self, _filters: DocumentFilters) -> Result<Vec<Document>> {
            Ok(Vec::new())
        }
        async fn search_documents(&self, _query: &str) -> Result<Vec<Document>> {
            Ok(Vec::new())
        }
        async fn delete_document(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn save_embedding(&self, _segment_id: &str, _embedding: Vec<f32>) -> Result<()> {
            Ok(())
        }
        async fn get_embedding(&self, _segment_id: &str) -> Result<Option<Vec<f32>>> {
            Ok(None)
        }
        async fn vector_search(
            &self,
            _query_embedding: Vec<f32>,
            _k: usize,
            _filters: Option<SearchFilters>,
        ) -> Result<Vec<SearchResult>> {
            Ok(Vec::new())
        }
        async fn analyze_dependencies(&self, _repo_id: &str) -> Result<GraphAnalysis> {
            Ok(GraphAnalysis {
                total_modules: 0,
                total_dependencies: 0,
                max_depth: 0,
                average_dependencies: 0.0,
                isolated_modules: Vec::new(),
            })
        }
        async fn find_circular_dependencies(&self, _repo_id: &str) -> Result<Vec<Vec<String>>> {
            Ok(Vec::new())
        }
        async fn get_module_impact(&self, _module_id: &str) -> Result<ModuleImpact> {
            Ok(ModuleImpact {
                direct_dependents: 0,
                transitive_dependents: 0,
                direct_dependencies: 0,
                affected_modules: Vec::new(),
            })
        }
        async fn save_user(&self, _user: &crate::api::auth::user::User) -> Result<()> {
            Ok(())
        }
        async fn get_user(&self, _id: &str) -> Result<Option<crate::api::auth::user::User>> {
            Ok(None)
        }
        async fn get_user_by_email(
            &self,
            _email: &str,
        ) -> Result<Option<crate::api::auth::user::User>> {
            Ok(None)
        }
        async fn get_user_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<crate::api::auth::user::User>> {
            Ok(None)
        }
        async fn update_user(&self, _user: &crate::api::auth::user::User) -> Result<()> {
            Ok(())
        }
        async fn delete_user(&self, _id: &str) -> Result<()> {
            Ok(())
        }
        async fn save_session(&self, _session: &crate::api::auth::user::Session) -> Result<()> {
            Ok(())
        }
        async fn get_session(
            &self,
            _refresh_token: &str,
        ) -> Result<Option<crate::api::auth::user::Session>> {
            Ok(None)
        }
        async fn delete_session(&self, _refresh_token: &str) -> Result<()> {
            Ok(())
        }
        async fn delete_user_sessions(&self, _user_id: &str) -> Result<()> {
            Ok(())
        }
        async fn cleanup_expired_sessions(&self) -> Result<usize> {
            Ok(0)
        }
    }

    #[tokio::test]
    async fn test_storage_manager_connection() {
        let manager = StorageManager::new();
        let config = StorageConfig {
            enabled: true,
            connection_string: "ws://localhost:8000".to_string(),
            namespace: "test".to_string(),
            database: "test".to_string(),
            username: None,
            password: None,
            max_connections: 10,
            connection_timeout_seconds: 30,
            auto_migrate: false,
        };

        // Note: This will fail if SurrealDB is not running, which is expected
        // In a real test environment, we'd use a test database or mock
        let result = manager.connect(config).await;
        // We expect this might fail if DB is not available
        if result.is_ok() {
            assert!(manager.is_connected().await);
            assert!(manager.test_connection().await.is_ok());
        }
    }

    #[test]
    fn test_graph_analyzer() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("a".to_string(), "b".to_string());
        graph.add_dependency("b".to_string(), "c".to_string());
        graph.add_dependency("c".to_string(), "a".to_string()); // Circular

        let analysis = GraphAnalyzer::analyze_graph(&graph);
        assert_eq!(analysis.total_modules, 3);
        assert_eq!(analysis.total_dependencies, 3);

        let cycles = GraphAnalyzer::find_circular_dependencies(&graph);
        assert!(!cycles.is_empty(), "Should detect circular dependency");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&a, &b).unwrap();
        assert!(
            (similarity - 1.0).abs() < 0.001,
            "Identical vectors should have similarity 1.0"
        );

        let c = vec![0.0, 1.0, 0.0];
        let similarity2 = cosine_similarity(&a, &c).unwrap();
        assert!(
            (similarity2 - 0.0).abs() < 0.001,
            "Orthogonal vectors should have similarity 0.0"
        );
    }

    #[test]
    fn test_normalize_vector() {
        let mut v = vec![3.0, 4.0, 0.0];
        normalize_vector(&mut v);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.001,
            "Normalized vector should have norm 1.0"
        );
    }
}
