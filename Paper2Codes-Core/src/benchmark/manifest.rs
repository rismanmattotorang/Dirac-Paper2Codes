//! Benchmark dataset manifest.
//!
//! A manifest is a small JSON file describing a curated set of paper→repo cases
//! (PaperBench-style). Each case points at a paper and, optionally, an
//! author-released reference repository (for reference-based scoring) and/or a
//! rubric (for reference-free LLM-judge scoring).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Paper2CodesError, Result};

/// A single benchmark case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCase {
    /// Stable identifier (used in the report).
    pub id: String,
    /// Path to the paper (PDF or text).
    pub paper_path: PathBuf,
    /// Optional author-released reference repository for reference-based scoring.
    #[serde(default)]
    pub reference_repo: Option<PathBuf>,
    /// Optional rubric file (JSON) for reference-free judge scoring.
    #[serde(default)]
    pub rubric_path: Option<PathBuf>,
    /// Optional domain skill id to specialise generation for this case. When
    /// absent, generators may auto-suggest a skill from the paper text.
    #[serde(default)]
    pub skill_id: Option<String>,
}

/// A benchmark manifest: an ordered set of cases plus optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkManifest {
    #[serde(default)]
    pub name: Option<String>,
    pub cases: Vec<BenchmarkCase>,
}

impl BenchmarkManifest {
    /// Parse a manifest from a JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            Paper2CodesError::Config(ConfigError::Invalid(format!(
                "Invalid benchmark manifest JSON: {}",
                e
            )))
        })
    }

    /// Load a manifest from a JSON file. Relative `paper_path` /
    /// `reference_repo` / `rubric_path` entries are resolved against the
    /// manifest file's directory so manifests are portable.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            Paper2CodesError::Config(ConfigError::Invalid(format!(
                "Failed to read manifest {}: {}",
                path.display(),
                e
            )))
        })?;
        let mut manifest = Self::from_json(&content)?;

        if let Some(base) = path.parent() {
            for case in &mut manifest.cases {
                case.paper_path = resolve(base, &case.paper_path);
                case.reference_repo = case.reference_repo.as_ref().map(|p| resolve(base, p));
                case.rubric_path = case.rubric_path.as_ref().map(|p| resolve(base, p));
            }
        }
        Ok(manifest)
    }
}

/// Resolve `path` against `base` unless it is already absolute.
fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let json = r#"{
            "name": "smoke",
            "cases": [
                { "id": "attention", "paper_path": "papers/attention.txt" },
                { "id": "resnet", "paper_path": "papers/resnet.txt",
                  "reference_repo": "refs/resnet", "rubric_path": "rubrics/resnet.json" }
            ]
        }"#;
        let m = BenchmarkManifest::from_json(json).unwrap();
        assert_eq!(m.name.as_deref(), Some("smoke"));
        assert_eq!(m.cases.len(), 2);
        assert_eq!(m.cases[0].id, "attention");
        assert!(m.cases[0].reference_repo.is_none());
        assert_eq!(
            m.cases[1].reference_repo.as_ref().unwrap(),
            &PathBuf::from("refs/resnet")
        );
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(BenchmarkManifest::from_json("{ not json ").is_err());
    }

    #[test]
    fn resolve_handles_absolute_and_relative() {
        let base = Path::new("/data/bench");
        assert_eq!(
            resolve(base, Path::new("papers/p.txt")),
            PathBuf::from("/data/bench/papers/p.txt")
        );
        assert_eq!(
            resolve(base, Path::new("/abs/p.txt")),
            PathBuf::from("/abs/p.txt")
        );
    }
}
