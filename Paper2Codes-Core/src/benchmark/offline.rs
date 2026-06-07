//! Offline (no-LLM) benchmark generator.
//!
//! Lets the benchmark harness, dataset, and scoring be exercised end-to-end
//! *before any API keys are configured*. For each case it selects the domain
//! skill (explicit `skill_id`, else auto-matched from the paper text) and emits
//! a deterministic `main.py` skeleton importing the skill's recommended
//! libraries. The output is intentionally a stub — this validates the pipeline
//! plumbing (manifest → generate → reference scoring → report), not LLM quality.

use async_trait::async_trait;

use super::{BenchmarkCase, GeneratedRepo, RepoGenerator};
use crate::error::Result;
use crate::skills::SkillRegistry;

/// Deterministic, network-free [`RepoGenerator`] for smoke testing.
pub struct OfflineStubGenerator {
    pub language: String,
}

impl OfflineStubGenerator {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
        }
    }
}

impl Default for OfflineStubGenerator {
    fn default() -> Self {
        Self::new("python")
    }
}

#[async_trait]
impl RepoGenerator for OfflineStubGenerator {
    async fn generate(&self, case: &BenchmarkCase) -> Result<GeneratedRepo> {
        let registry = SkillRegistry::with_builtins();
        let paper_text = std::fs::read_to_string(&case.paper_path).unwrap_or_default();

        let skill = case
            .skill_id
            .as_deref()
            .and_then(|id| registry.get(id))
            .or_else(|| registry.match_skill(&paper_text));

        let (skill_name, libraries) = match skill {
            Some(s) => (s.name.clone(), s.libraries_for(&self.language)),
            None => ("Generic".to_string(), Vec::new()),
        };

        let title = paper_text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("paper")
            .trim()
            .to_string();

        let imports = libraries
            .iter()
            .map(|lib| format!("import {}", lib))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!(
            "# Offline stub for: {}\n# Domain skill: {}\n{}\n\n\ndef main():\n    \"\"\"Offline stub (no LLM). Provide API keys to generate the real implementation.\"\"\"\n    raise NotImplementedError\n",
            title, skill_name, imports
        );

        Ok(GeneratedRepo {
            files: vec![("main.py".to_string(), content)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn offline_generator_uses_explicit_skill() {
        let gen = OfflineStubGenerator::new("python");
        let case = BenchmarkCase {
            id: "c1".into(),
            paper_path: "/nonexistent/paper.txt".into(),
            reference_repo: None,
            rubric_path: None,
            skill_id: Some("quantum-computation".into()),
        };
        let repo = gen.generate(&case).await.unwrap();
        assert_eq!(repo.files.len(), 1);
        let (path, content) = &repo.files[0];
        assert_eq!(path, "main.py");
        assert!(content.contains("Quantum Computation"));
        assert!(content.contains("import qiskit"));
    }

    #[tokio::test]
    async fn offline_generator_is_deterministic() {
        let gen = OfflineStubGenerator::new("python");
        let case = BenchmarkCase {
            id: "c1".into(),
            paper_path: "/nonexistent/paper.txt".into(),
            reference_repo: None,
            rubric_path: None,
            skill_id: Some("computational-finance".into()),
        };
        let a = gen.generate(&case).await.unwrap();
        let b = gen.generate(&case).await.unwrap();
        assert_eq!(a.files, b.files);
    }
}
