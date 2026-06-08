//! Reproducibility benchmark harness (Phase 5).
//!
//! Turns "Dirac-Paper2Codes is superior" from a claim into a measurement. The
//! harness runs a curated set of paper→repo cases (see [`manifest`]) through a
//! pluggable [`RepoGenerator`], then scores each generated repository two ways:
//!
//! * **reference-based** — against an author-released repository
//!   ([`scoring::reference_score`]: file precision/recall/F1 + content
//!   similarity), and
//! * **reference-free** — via an optional LLM-judge [`RubricGrader`].
//!
//! Results aggregate into a [`BenchmarkReport`] (Markdown / JSON). The generator
//! and grader are traits, so the whole pipeline is unit-tested with mocks and
//! the production path wraps the real coordinator + judge in the CLI.

pub mod manifest;
pub mod offline;
pub mod rubric;
pub mod scoring;

pub use manifest::{BenchmarkCase, BenchmarkManifest};
pub use offline::OfflineStubGenerator;
pub use rubric::LlmRubricGrader;
pub use scoring::{FileSetScore, ReferenceScore};

use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::types::Repository;

/// A generated repository reduced to `(relative_path, content)` text files.
#[derive(Debug, Clone, Default)]
pub struct GeneratedRepo {
    pub files: Vec<(String, String)>,
}

impl GeneratedRepo {
    /// Build from an in-memory [`Repository`] (the coordinator's output).
    pub fn from_repository(repo: &Repository) -> Self {
        let files = repo
            .modules
            .iter()
            .map(|m| (m.file_path.to_string_lossy().to_string(), m.content.clone()))
            .collect();
        Self { files }
    }

    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|(p, _)| p.clone()).collect()
    }
}

/// Produces a generated repository for a benchmark case (e.g. by running the
/// coordinator on the paper).
#[async_trait]
pub trait RepoGenerator: Send + Sync {
    async fn generate(&self, case: &BenchmarkCase) -> Result<GeneratedRepo>;
}

/// Produces a reference-free rubric score in `[0, 1]` for a generated repo.
#[async_trait]
pub trait RubricGrader: Send + Sync {
    async fn grade(&self, case: &BenchmarkCase, repo: &GeneratedRepo) -> Result<f64>;
}

/// Per-case benchmark result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseResult {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_overall: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_f1: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_similarity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubric_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregated metrics across all cases.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregateMetrics {
    pub case_count: usize,
    pub scored_count: usize,
    pub mean_file_f1: f32,
    pub mean_content_similarity: f32,
    pub mean_reference_overall: f32,
    pub mean_rubric: f32,
    pub error_count: usize,
}

/// Full benchmark report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub cases: Vec<CaseResult>,
    pub aggregate: AggregateMetrics,
}

impl BenchmarkReport {
    /// Aggregate per-case results into a report.
    pub fn from_cases(name: Option<String>, cases: Vec<CaseResult>) -> Self {
        let file_f1: Vec<f32> = cases.iter().filter_map(|c| c.file_f1).collect();
        let content: Vec<f32> = cases.iter().filter_map(|c| c.content_similarity).collect();
        let overall: Vec<f32> = cases.iter().filter_map(|c| c.reference_overall).collect();
        let rubric: Vec<f32> = cases
            .iter()
            .filter_map(|c| c.rubric_score.map(|r| r as f32))
            .collect();
        let error_count = cases.iter().filter(|c| c.error.is_some()).count();
        let scored_count = cases
            .iter()
            .filter(|c| c.reference_overall.is_some() || c.rubric_score.is_some())
            .count();

        let aggregate = AggregateMetrics {
            case_count: cases.len(),
            scored_count,
            mean_file_f1: scoring::mean(&file_f1),
            mean_content_similarity: scoring::mean(&content),
            mean_reference_overall: scoring::mean(&overall),
            mean_rubric: scoring::mean(&rubric),
            error_count,
        };

        Self {
            name,
            cases,
            aggregate,
        }
    }

    /// Render a human-readable Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Benchmark Report{}\n\n",
            self.name
                .as_ref()
                .map(|n| format!(": {}", n))
                .unwrap_or_default()
        ));
        out.push_str("| Case | Ref overall | File F1 | Content sim | Rubric | Error |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for c in &self.cases {
            let fmt_f32 = |v: Option<f32>| v.map(|x| format!("{:.3}", x)).unwrap_or_else(|| "—".into());
            let fmt_f64 = |v: Option<f64>| v.map(|x| format!("{:.3}", x)).unwrap_or_else(|| "—".into());
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                c.id,
                fmt_f32(c.reference_overall),
                fmt_f32(c.file_f1),
                fmt_f32(c.content_similarity),
                fmt_f64(c.rubric_score),
                c.error.as_deref().unwrap_or("")
            ));
        }
        let a = &self.aggregate;
        out.push_str(&format!(
            "\n**Aggregate** ({} cases, {} scored, {} errors):\n\
             - Mean reference overall: {:.3}\n\
             - Mean file F1: {:.3}\n\
             - Mean content similarity: {:.3}\n\
             - Mean rubric: {:.3}\n",
            a.case_count,
            a.scored_count,
            a.error_count,
            a.mean_reference_overall,
            a.mean_file_f1,
            a.mean_content_similarity,
            a.mean_rubric,
        ));
        out
    }

    /// Serialise to pretty JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            crate::error::Paper2CodesError::Validation(format!("Failed to serialize report: {}", e))
        })
    }

    /// Parse a report from JSON (e.g. a stored baseline).
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::error::Paper2CodesError::Validation(format!("Failed to parse report: {}", e))
        })
    }

    /// Evaluate the report against a quality gate, returning a list of human
    /// readable violations (empty = pass). Used to gate releases on benchmark
    /// quality and guard against regressions vs. a baseline.
    pub fn gate_violations(
        &self,
        gate: &QualityGate,
        baseline: Option<&BenchmarkReport>,
    ) -> Vec<String> {
        let a = &self.aggregate;
        let mut violations = Vec::new();

        if let Some(min) = gate.min_reference_overall {
            if a.mean_reference_overall < min {
                violations.push(format!(
                    "mean reference-overall {:.3} < required {:.3}",
                    a.mean_reference_overall, min
                ));
            }
        }
        if let Some(min) = gate.min_rubric {
            if a.mean_rubric < min {
                violations.push(format!(
                    "mean rubric {:.3} < required {:.3}",
                    a.mean_rubric, min
                ));
            }
        }
        if !gate.allow_errors && a.error_count > 0 {
            violations.push(format!("{} case(s) errored", a.error_count));
        }

        // No-regression check against a baseline (within tolerance).
        if let Some(base) = baseline {
            let tol = gate.regression_tolerance;
            if a.mean_reference_overall + tol < base.aggregate.mean_reference_overall {
                violations.push(format!(
                    "reference-overall regressed {:.3} -> {:.3} (tolerance {:.3})",
                    base.aggregate.mean_reference_overall, a.mean_reference_overall, tol
                ));
            }
            if a.mean_rubric + tol < base.aggregate.mean_rubric {
                violations.push(format!(
                    "rubric regressed {:.3} -> {:.3} (tolerance {:.3})",
                    base.aggregate.mean_rubric, a.mean_rubric, tol
                ));
            }
        }

        violations
    }
}

/// Release quality gate thresholds (see PRODUCTION_PLAN.md §7).
#[derive(Debug, Clone)]
pub struct QualityGate {
    pub min_reference_overall: Option<f32>,
    pub min_rubric: Option<f32>,
    /// Fail the gate if any case errored.
    pub allow_errors: bool,
    /// Permitted drop vs. a baseline before flagging a regression.
    pub regression_tolerance: f32,
}

impl Default for QualityGate {
    fn default() -> Self {
        Self {
            min_reference_overall: None,
            min_rubric: None,
            allow_errors: false,
            regression_tolerance: 0.02,
        }
    }
}

/// Load a repository on disk into `(relative_path, content)` text files.
///
/// Skips hidden entries, common non-source directories, oversized files, and
/// files that are not valid UTF-8 (binaries).
pub fn load_repo_files(root: impl AsRef<Path>) -> Result<Vec<(String, String)>> {
    let root = root.as_ref();
    const MAX_BYTES: u64 = 512 * 1024;
    const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "__pycache__", ".venv", "venv"];

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(name.starts_with('.') && e.depth() > 0) && !SKIP_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_BYTES {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            let rel = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .to_string();
            files.push((rel, content));
        }
    }
    Ok(files)
}

/// Run the benchmark: generate a repo per case, then score it reference-based
/// (when a reference repo is present) and reference-free (when a grader is
/// supplied). Generation/scoring errors are captured per case, never aborting
/// the whole run.
pub async fn run_benchmark(
    manifest: &BenchmarkManifest,
    generator: &dyn RepoGenerator,
    grader: Option<&dyn RubricGrader>,
) -> BenchmarkReport {
    let mut results = Vec::with_capacity(manifest.cases.len());

    for case in &manifest.cases {
        let mut result = CaseResult {
            id: case.id.clone(),
            reference_overall: None,
            file_f1: None,
            content_similarity: None,
            rubric_score: None,
            error: None,
        };

        match generator.generate(case).await {
            Ok(repo) => {
                // Reference-based scoring.
                if let Some(ref_path) = &case.reference_repo {
                    match load_repo_files(ref_path) {
                        Ok(ref_files) => {
                            let score = scoring::reference_score(&repo.files, &ref_files);
                            result.reference_overall = Some(score.overall);
                            result.file_f1 = Some(score.file.f1);
                            result.content_similarity = Some(score.content_similarity);
                        }
                        Err(e) => {
                            result.error = Some(format!("reference load failed: {}", e));
                        }
                    }
                }
                // Reference-free rubric scoring.
                if let Some(grader) = grader {
                    match grader.grade(case, &repo).await {
                        Ok(score) => result.rubric_score = Some(score),
                        Err(e) => {
                            let msg = format!("rubric grading failed: {}", e);
                            result.error = Some(match result.error.take() {
                                Some(prev) => format!("{}; {}", prev, msg),
                                None => msg,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                result.error = Some(format!("generation failed: {}", e));
            }
        }

        results.push(result);
    }

    BenchmarkReport::from_cases(manifest.name.clone(), results)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockGenerator;
    #[async_trait]
    impl RepoGenerator for MockGenerator {
        async fn generate(&self, case: &BenchmarkCase) -> Result<GeneratedRepo> {
            if case.id == "boom" {
                return Err(crate::error::Paper2CodesError::Validation("boom".into()));
            }
            Ok(GeneratedRepo {
                files: vec![("model.py".to_string(), "def forward(): pass".to_string())],
            })
        }
    }

    struct MockGrader;
    #[async_trait]
    impl RubricGrader for MockGrader {
        async fn grade(&self, _case: &BenchmarkCase, repo: &GeneratedRepo) -> Result<f64> {
            // Score = fraction of a non-empty repo, trivially deterministic.
            Ok(if repo.files.is_empty() { 0.0 } else { 0.8 })
        }
    }

    fn case(id: &str) -> BenchmarkCase {
        BenchmarkCase {
            id: id.to_string(),
            paper_path: "p.txt".into(),
            reference_repo: None,
            rubric_path: None,
            skill_id: None,
        }
    }

    #[tokio::test]
    async fn runs_cases_and_aggregates_rubric() {
        let manifest = BenchmarkManifest {
            name: Some("t".into()),
            cases: vec![case("a"), case("b")],
        };
        let report = run_benchmark(&manifest, &MockGenerator, Some(&MockGrader)).await;
        assert_eq!(report.aggregate.case_count, 2);
        assert_eq!(report.aggregate.error_count, 0);
        assert!((report.aggregate.mean_rubric - 0.8).abs() < 1e-6);
    }

    #[tokio::test]
    async fn captures_generation_errors_without_aborting() {
        let manifest = BenchmarkManifest {
            name: None,
            cases: vec![case("a"), case("boom"), case("b")],
        };
        let report = run_benchmark(&manifest, &MockGenerator, Some(&MockGrader)).await;
        assert_eq!(report.aggregate.case_count, 3);
        assert_eq!(report.aggregate.error_count, 1);
        let boom = report.cases.iter().find(|c| c.id == "boom").unwrap();
        assert!(boom.error.as_ref().unwrap().contains("generation failed"));
        assert!(boom.rubric_score.is_none());
    }

    #[tokio::test]
    async fn reference_scoring_runs_against_temp_repo() {
        // Write a reference repo to a temp dir matching the generated file.
        let dir = std::env::temp_dir().join(format!("p2c_bench_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("model.py"), "def forward(): pass").unwrap();

        let mut c = case("a");
        c.reference_repo = Some(dir.clone());
        let manifest = BenchmarkManifest {
            name: None,
            cases: vec![c],
        };
        let report = run_benchmark(&manifest, &MockGenerator, None).await;
        let result = &report.cases[0];
        assert_eq!(result.file_f1, Some(1.0));
        assert_eq!(result.content_similarity, Some(1.0));
        assert_eq!(result.reference_overall, Some(1.0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn report_renders_markdown_and_json() {
        let report = BenchmarkReport::from_cases(
            Some("demo".into()),
            vec![CaseResult {
                id: "a".into(),
                reference_overall: Some(0.5),
                file_f1: Some(0.6),
                content_similarity: Some(0.4),
                rubric_score: Some(0.7),
                error: None,
            }],
        );
        let md = report.to_markdown();
        assert!(md.contains("Benchmark Report: demo"));
        assert!(md.contains("Mean reference overall: 0.500"));
        let json = report.to_json().unwrap();
        assert!(json.contains("\"mean_rubric\""));
        // Round-trips back from JSON (baseline loading).
        let parsed = BenchmarkReport::from_json(&json).unwrap();
        assert_eq!(parsed.aggregate.case_count, 1);
    }

    fn report_with(overall: f32, rubric: f32, errors: usize) -> BenchmarkReport {
        let mut cases = vec![CaseResult {
            id: "a".into(),
            reference_overall: Some(overall),
            file_f1: Some(overall),
            content_similarity: Some(overall),
            rubric_score: Some(rubric as f64),
            error: None,
        }];
        for i in 0..errors {
            cases.push(CaseResult {
                id: format!("err{}", i),
                reference_overall: None,
                file_f1: None,
                content_similarity: None,
                rubric_score: None,
                error: Some("boom".into()),
            });
        }
        BenchmarkReport::from_cases(None, cases)
    }

    #[test]
    fn gate_passes_when_thresholds_met() {
        let report = report_with(0.8, 0.9, 0);
        let gate = QualityGate {
            min_reference_overall: Some(0.7),
            min_rubric: Some(0.7),
            allow_errors: false,
            regression_tolerance: 0.02,
        };
        assert!(report.gate_violations(&gate, None).is_empty());
    }

    #[test]
    fn gate_flags_low_scores_and_errors() {
        let report = report_with(0.5, 0.4, 1);
        let gate = QualityGate {
            min_reference_overall: Some(0.7),
            min_rubric: Some(0.7),
            allow_errors: false,
            regression_tolerance: 0.02,
        };
        let v = report.gate_violations(&gate, None);
        assert_eq!(v.len(), 3); // overall, rubric, errors
        assert!(v.iter().any(|s| s.contains("reference-overall")));
        assert!(v.iter().any(|s| s.contains("rubric")));
        assert!(v.iter().any(|s| s.contains("errored")));
    }

    #[test]
    fn gate_detects_regression_vs_baseline() {
        let baseline = report_with(0.80, 0.80, 0);
        let current = report_with(0.70, 0.80, 0); // 0.10 drop > 0.02 tol
        let gate = QualityGate {
            min_reference_overall: None,
            min_rubric: None,
            allow_errors: true,
            regression_tolerance: 0.02,
        };
        let v = current.gate_violations(&gate, Some(&baseline));
        assert!(v.iter().any(|s| s.contains("regressed")));

        // A small drop within tolerance is fine.
        let small = report_with(0.79, 0.80, 0);
        assert!(small.gate_violations(&gate, Some(&baseline)).is_empty());
    }
}
