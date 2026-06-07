//! Paper2Codes: Neuro-Symbolic RAG Framework for Automated Code Generation
//!
//! Paper2Codes is a production-ready framework that automatically generates executable code
//! and documentation from scientific literature. It leverages state-of-the-art LLMs, advanced
//! retrieval algorithms, and symbolic verification.
//!
//! # Features
//!
//! - **Advanced RAG**: Contextual Paper Retrieval with hybrid semantic+keyword scoring
//! - **Multi-Agent Architecture**: Specialized agents for planning, analysis, coding, and verification
//! - **Symbolically-Augmented Verification**: Multi-layered code verification
//! - **Production-Ready**: Comprehensive error handling, caching, parallel execution
//! - **Persistent Storage**: SurrealDB integration with graph-based dependency tracking
//!
//! # Quick Start
//!
//! ```no_run
//! use paper2codes::config::Config;
//! use paper2codes::coordinator::Coordinator;
//! use paper2codes::document::DocumentProcessor;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = Config::default();
//!     
//!     // Initialize coordinator
//!     let mut coordinator = Coordinator::new(config).await?;
//!     
//!     // Process paper
//!     let processor = DocumentProcessor::new();
//!     let paper = processor.parse_text("Paper content...", Some("Title".to_string())).await?;
//!     
//!     // Generate code
//!     let repository = coordinator.process_paper(paper).await?;
//!     
//!     println!("Generated {} modules", repository.modules.len());
//!     Ok(())
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`agents`]: Multi-agent system (Planning, Analysis, Coding, Verification)
//! - [`config`]: Configuration management
//! - [`coordinator`]: Central orchestration service
//! - [`document`]: Paper parsing and processing
//! - [`error`]: Error handling and recovery
//! - [`execution`]: Code execution and sandboxing
//! - [`llm`]: LLM integration and routing
//! - [`retrieval`]: Advanced RAG and CPR engine
//! - [`storage`]: Persistent storage with SurrealDB
//! - [`symbolic`]: Symbolic reasoning (SMT, CAS)
//! - [`types`]: Core data types
//! - [`ui`]: Terminal user interface
//! - [`verification`]: SACV verification pipeline

pub mod agents;
#[cfg(feature = "api")]
pub mod api;
pub mod benchmark;
pub mod config;
pub mod coordinator;
pub mod document;
pub mod domain;
pub mod error;
pub mod evaluation;
pub mod execution;
pub mod github;
pub mod llm;
pub mod performance;
pub mod prompts;
pub mod recovery;
pub mod reproduction;
pub mod retrieval;
pub mod skills;
pub mod storage;
pub mod symbolic;
pub mod types;
pub mod ui;
pub mod verification;

// Re-export commonly used types
pub use config::Config;
pub use coordinator::Coordinator;
pub use error::{Paper2CodesError, Result};
pub use types::{Paper, Repository, Task, TaskStatus, TaskType};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Get library information
pub fn info() -> String {
    format!("{} v{}", NAME, VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info() {
        let info = info();
        assert!(info.contains("paper2codes"));
    }

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
