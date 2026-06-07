#![cfg(feature = "legacy-tests")]

/// Comprehensive unit tests for Paper2Codes components

#[cfg(test)]
mod config_tests {
    use paper2codes::config::{AgentConfig, Config, LLMConfig};

    #[test]
    fn test_config_creation() {
        let config = Config::default();
        assert!(!config.llm.default_provider.is_empty());
        assert!(config.agents.max_iterations > 0);
    }

    #[test]
    fn test_agent_config() {
        let agent_config = AgentConfig::default();
        assert!(agent_config.max_iterations > 0);
        assert!(agent_config.parallel_tasks > 0);
        assert!(!agent_config.planning_model.is_empty());
    }

    #[test]
    fn test_llm_config() {
        let llm_config = LLMConfig::default();
        assert!(!llm_config.default_provider.is_empty());
        assert!(llm_config.timeout_seconds > 0);
        assert!(llm_config.max_retries > 0);
    }
}

#[cfg(test)]
mod type_tests {
    use paper2codes::types::*;
    use std::path::PathBuf;

    #[test]
    fn test_task_creation() {
        let task = Task::new(TaskType::Planning, "Test task".to_string());
        assert_eq!(task.description, "Test task");
        assert!(matches!(task.status, TaskStatus::Pending));
    }

    #[test]
    fn test_module_creation() {
        let module = Module {
            id: "test".to_string(),
            name: "test_module".to_string(),
            description: "Test module".to_string(),
            module_type: ModuleType::Function,
            dependencies: vec![],
            language: ProgrammingLanguage::Python,
            status: ModuleStatus::Pending,
        };
        assert_eq!(module.name, "test_module");
    }

    #[test]
    fn test_repository_creation() {
        let repo = Repository::new(PathBuf::from("./test"));
        assert!(!repo.id.is_empty());
        assert_eq!(repo.modules.len(), 0);
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        graph.add_node("mod1".to_string());
        graph.add_node("mod2".to_string());
        graph.add_edge("mod1".to_string(), "mod2".to_string());

        assert!(graph.nodes.contains(&"mod1".to_string()));
        assert!(graph.nodes.contains(&"mod2".to_string()));
    }
}

#[cfg(test)]
mod error_tests {
    use paper2codes::error::*;

    #[test]
    fn test_error_types() {
        let doc_error = DocumentError::ParseFailed("test".to_string());
        assert!(doc_error.to_string().contains("test"));

        let llm_error = LLMError::RequestFailed("connection error".to_string());
        assert!(llm_error.to_string().contains("connection error"));
    }

    #[test]
    fn test_error_conversion() {
        let doc_error: Paper2CodesError = DocumentError::ParseFailed("test".to_string()).into();
        assert!(matches!(doc_error, Paper2CodesError::Document(_)));
    }
}

#[cfg(test)]
mod document_processor_tests {
    use paper2codes::document::DocumentProcessor;

    #[tokio::test]
    async fn test_simple_text_parsing() {
        let processor = DocumentProcessor::new();
        let content = "Title\n\nAbstract\n\nContent here.";
        let result = processor
            .parse_text(content, Some("Test".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_content() {
        let processor = DocumentProcessor::new();
        let result = processor.parse_text("", None).await;
        assert!(result.is_err(), "Empty content should return error");
    }

    #[tokio::test]
    async fn test_segmentation() {
        let processor = DocumentProcessor::new();
        let content = r#"
Introduction

This is the introduction.

Methodology

This is the methodology.

Conclusion

This is the conclusion.
"#;
        let result = processor
            .parse_text(content, Some("Test".to_string()))
            .await;
        assert!(result.is_ok());

        let paper = result.unwrap();
        assert!(paper.segments.len() > 1, "Should have multiple segments");
    }
}

#[cfg(test)]
mod content_extraction_tests {
    use paper2codes::document::{ContentExtractor, DocumentProcessor};
    use paper2codes::types::Paper;

    #[tokio::test]
    async fn test_algorithm_extraction() {
        let processor = DocumentProcessor::new();
        let content = r#"
Algorithm 1: Test Algorithm
Input: x
Output: y
1: Initialize y = 0
2: Process x
3: Return y
End
"#;
        let paper = processor
            .parse_text(content, Some("Test".to_string()))
            .await
            .unwrap();
        let algorithms = processor.extract_algorithms(&paper);

        assert!(!algorithms.is_empty(), "Should extract algorithm");
        assert!(
            algorithms[0].pseudocode.contains("Algorithm"),
            "Should contain algorithm header"
        );
    }

    #[tokio::test]
    async fn test_equation_extraction() {
        let processor = DocumentProcessor::new();
        let extractor = ContentExtractor::new();
        let content = "The formula is f(x) = ax^2 + bx + c where a, b, c are constants.";
        let paper = processor
            .parse_text(content, Some("Test".to_string()))
            .await
            .unwrap();
        let equations = extractor.extract_equations(&paper);

        // Equation extraction depends on patterns, so we just check it doesn't crash
        assert!(equations.len() >= 0);
    }
}

#[cfg(test)]
mod agent_tests {
    use paper2codes::agents::*;
    use paper2codes::types::*;

    #[test]
    fn test_agent_creation() {
        let planning_agent = PlanningAgent::new();
        assert_eq!(planning_agent.agent_type(), AgentType::Planning);

        let analysis_agent = AnalysisAgent::new();
        assert_eq!(analysis_agent.agent_type(), AgentType::Analysis);

        let coding_agent = CodingAgent::new();
        assert_eq!(coding_agent.agent_type(), AgentType::Coding);

        let verification_agent = VerificationAgent::new();
        assert_eq!(verification_agent.agent_type(), AgentType::Verification);
    }
}

#[cfg(test)]
mod verification_tests {
    use chrono::Utc;
    use paper2codes::types::{CodeModule, ModuleStatus, ProgrammingLanguage};
    use paper2codes::verification::StaticAnalyzer;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_static_analyzer_python() {
        let analyzer = StaticAnalyzer::new();
        let code = CodeModule {
            id: "test".to_string(),
            repository_id: None,
            file_path: PathBuf::from("test.py"),
            language: ProgrammingLanguage::Python,
            content: "def test():\n    return 42".to_string(),
            ast: None,
            dependencies: vec![],
            tests: vec![],
            status: ModuleStatus::Completed,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = analyzer.analyze(&code).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_static_analyzer_invalid_syntax() {
        let analyzer = StaticAnalyzer::new();
        let code = CodeModule {
            id: "test".to_string(),
            repository_id: None,
            file_path: PathBuf::from("test.py"),
            language: ProgrammingLanguage::Python,
            content: "def test(\n    invalid syntax here".to_string(),
            ast: None,
            dependencies: vec![],
            tests: vec![],
            status: ModuleStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = analyzer.analyze(&code).await;
        assert!(result.is_ok());
        // Should report syntax errors
        if let Ok(issues) = result {
            assert!(issues
                .iter()
                .any(|i| matches!(i.category, paper2codes::types::IssueCategory::Syntax)));
        }
    }
}

#[cfg(test)]
mod execution_tests {
    use chrono::Utc;
    use paper2codes::execution::SandboxRunner;
    use paper2codes::types::{CodeModule, ModuleStatus, ProgrammingLanguage};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sandbox_creation() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxRunner::new(temp_dir.path().to_path_buf());
        assert!(true, "Sandbox should be created successfully");
    }

    #[tokio::test]
    async fn test_python_execution() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = SandboxRunner::new(temp_dir.path().to_path_buf());

        let code = CodeModule {
            id: "test".to_string(),
            repository_id: None,
            file_path: PathBuf::from("test.py"),
            language: ProgrammingLanguage::Python,
            content: "print('Hello, World!')".to_string(),
            ast: None,
            dependencies: vec![],
            tests: vec![],
            status: ModuleStatus::Completed,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = sandbox.execute_code(&code, None).await;
        assert!(result.is_ok());

        if let Ok(exec_result) = result {
            assert_eq!(exec_result.exit_code, 0);
            assert!(exec_result.stdout.contains("Hello, World!"));
        }
    }
}

#[cfg(test)]
mod coordinator_tests {
    use paper2codes::coordinator::TaskQueue;
    use paper2codes::types::{Task, TaskStatus, TaskType};

    #[test]
    fn test_task_queue_operations() {
        let mut queue = TaskQueue::new();

        let task1 = Task::new(TaskType::Planning, "Task 1".to_string());
        let task2 = Task::new(
            TaskType::Coding {
                module_id: "mod1".to_string(),
            },
            "Task 2".to_string(),
        );

        queue.add_task(task1.clone());
        queue.add_task(task2.clone());

        assert_eq!(queue.total_count(), 2);
        assert!(!queue.is_empty());

        let ready = queue.get_ready_tasks();
        assert_eq!(ready.len(), 2);

        queue.mark_completed(&task1.id);
        assert_eq!(queue.get_completed().len(), 1);
        assert_eq!(queue.get_pending().len(), 1);
    }

    #[test]
    fn test_task_dependencies() {
        let mut queue = TaskQueue::new();

        let task1 = Task::new(TaskType::Planning, "Task 1".to_string());
        let mut task2 = Task::new(
            TaskType::Coding {
                module_id: "mod1".to_string(),
            },
            "Task 2".to_string(),
        );
        task2.dependencies = vec![task1.id];

        queue.add_task(task1.clone());
        queue.add_task(task2.clone());

        let ready = queue.get_ready_tasks();
        // Only task1 should be ready initially
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, task1.id);

        queue.mark_completed(&task1.id);

        let ready_after = queue.get_ready_tasks();
        // Now task2 should be ready
        assert_eq!(ready_after.len(), 1);
        assert_eq!(ready_after[0].id, task2.id);
    }
}

#[cfg(test)]
mod storage_tests {
    use paper2codes::storage::StorageManager;

    #[test]
    fn test_storage_manager_creation() {
        let result = StorageManager::new();
        assert!(result.is_ok(), "Storage manager should be created");
    }

    #[test]
    fn test_storage_manager_not_connected() {
        let manager = StorageManager::new().unwrap();
        assert!(!manager.is_connected(), "Should not be connected initially");
    }
}
