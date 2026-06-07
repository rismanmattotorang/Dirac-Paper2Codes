//! Offline end-to-end pipeline tests.
//!
//! These run **without any API keys** and exercise the real engine plumbing:
//! document parsing → segmentation → hybrid retrieval (BM25 + RRF + structural
//! + CRAG), domain-skill selection, and the benchmark harness scoring over the
//! bundled dataset with the offline stub generator. They guard the main user
//! scenarios so the system can be validated before keys are configured.

use std::path::PathBuf;

use paper2codes::benchmark::{run_benchmark, BenchmarkManifest, OfflineStubGenerator};
use paper2codes::document::DocumentProcessor;
use paper2codes::retrieval::{CPREngine, DefaultCPREngine};
use paper2codes::skills::SkillRegistry;
use paper2codes::types::{Repository, Task, TaskType};

fn dataset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/dataset")
}

#[tokio::test]
async fn offline_paper_parsing_and_retrieval() {
    // 1. Parse a real bundled "paper" entirely offline (no LLM).
    let text = std::fs::read_to_string(dataset_dir().join("papers/black-scholes.txt")).unwrap();
    let processor = DocumentProcessor::new();
    let paper = processor
        .parse_text(&text, Some("Black-Scholes".to_string()))
        .await
        .expect("offline paper parsing should succeed");
    assert!(
        !paper.segments.is_empty(),
        "segmentation should produce segments"
    );

    // 2. Retrieve context for a coding task that references a structural anchor.
    let engine = DefaultCPREngine::new();
    let repo = Repository::new(PathBuf::from("./output"));
    let task = Task::new(
        TaskType::Coding {
            module_id: "pricing".to_string(),
        },
        "Implement the European call price using Equation (3)".to_string(),
    );

    let contexts = engine
        .retrieve(&task, &paper, &repo, 5)
        .await
        .expect("retrieval should succeed");

    assert!(
        !contexts.is_empty(),
        "hybrid retrieval should return relevant context"
    );
}

#[tokio::test]
async fn offline_benchmark_over_bundled_dataset() {
    // Run the whole benchmark harness offline over the curated dataset.
    let manifest = BenchmarkManifest::load(dataset_dir().join("manifest.json"))
        .expect("bundled manifest should load");
    assert_eq!(manifest.cases.len(), 3);

    let generator = OfflineStubGenerator::new("python");
    let report = run_benchmark(&manifest, &generator, None).await;

    // Every case ran, produced a repo, and was scored against its reference.
    assert_eq!(report.aggregate.case_count, 3);
    assert_eq!(report.aggregate.error_count, 0);
    assert_eq!(report.aggregate.scored_count, 3);

    for case in &report.cases {
        // The stub emits `main.py`, which every reference repo also contains,
        // so file-set F1 is perfect and content similarity is well-defined.
        assert_eq!(case.file_f1, Some(1.0), "case {} file F1", case.id);
        assert!(case.content_similarity.is_some(), "case {} content", case.id);
        assert!(case.reference_overall.unwrap() > 0.0);
    }

    // The report renders without panicking.
    assert!(report.to_markdown().contains("Benchmark Report"));
}

#[test]
fn all_built_in_domain_skills_are_well_formed() {
    let registry = SkillRegistry::with_builtins();
    assert!(registry.len() >= 8);
    for skill in registry.list() {
        skill.validate().expect("built-in skill is valid");
        // Default language primer is non-trivial and names the skill.
        let lang = skill.default_language();
        let primer = skill.system_prompt(lang);
        assert!(primer.contains(&skill.name));
        assert!(!skill.libraries_for(lang).is_empty(), "{} libs", skill.id);
    }
}
