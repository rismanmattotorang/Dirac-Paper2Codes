use crate::error::Result;
use crate::types::{DomainContext, Paper, ProgrammingLanguage};

pub struct DomainClassifier;

impl DomainClassifier {
    pub fn new() -> Self {
        Self
    }

    pub async fn classify(&self, paper: &Paper) -> Result<DomainContext> {
        // Simple keyword-based classification
        // In a real implementation, this would use an LLM or ML model

        let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();

        let domain = if self.contains_keywords(
            &text,
            &[
                "machine learning",
                "neural network",
                "deep learning",
                "ai",
                "artificial intelligence",
            ],
        ) {
            "Machine Learning"
        } else if self.contains_keywords(&text, &["optimization", "algorithm", "heuristic"]) {
            "Optimization"
        } else if self.contains_keywords(&text, &["database", "query", "sql", "data"]) {
            "Database"
        } else if self.contains_keywords(&text, &["compiler", "programming", "language", "syntax"])
        {
            "Programming Languages"
        } else if self.contains_keywords(&text, &["cryptography", "security", "encryption"]) {
            "Security"
        } else {
            "General"
        }
        .to_string();

        let language =
            if self.contains_keywords(&text, &["python", "numpy", "pytorch", "tensorflow"]) {
                ProgrammingLanguage::Python
            } else if self.contains_keywords(&text, &["rust", "cargo", "rustc"]) {
                ProgrammingLanguage::Rust
            } else {
                ProgrammingLanguage::Python // Default
            };

        let libraries = self.extract_libraries(&text);
        let tools = self.extract_tools(&text);

        Ok(DomainContext {
            domain,
            subdomain: None,
            language,
            libraries,
            tools,
        })
    }

    fn contains_keywords(&self, text: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|&keyword| text.contains(keyword))
    }

    fn extract_libraries(&self, text: &str) -> Vec<String> {
        let common_libraries = vec![
            "numpy",
            "pandas",
            "scipy",
            "scikit-learn",
            "pytorch",
            "tensorflow",
            "matplotlib",
            "seaborn",
            "jax",
            "flask",
            "django",
            "fastapi",
            "tokio",
            "serde",
            "reqwest",
            "clap",
            "ratatui",
        ];

        common_libraries
            .into_iter()
            .filter(|lib| text.contains(lib))
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_tools(&self, text: &str) -> Vec<String> {
        let common_tools = vec![
            "docker",
            "kubernetes",
            "git",
            "make",
            "cmake",
            "cargo",
            "pip",
            "conda",
            "venv",
            "poetry",
        ];

        common_tools
            .into_iter()
            .filter(|tool| text.contains(tool))
            .map(|s| s.to_string())
            .collect()
    }
}

impl Default for DomainClassifier {
    fn default() -> Self {
        Self::new()
    }
}
