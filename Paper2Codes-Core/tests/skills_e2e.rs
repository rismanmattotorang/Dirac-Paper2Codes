//! End-to-end test of the primary domain-skill user scenario:
//!
//!   choose a domain skill → upload a paper → skill-guided code generation →
//!   evaluate the generated code.
//!
//! The LLM-dependent generation step is replaced by a deterministic
//! `SkillGuidedGenerator` that consults the *real* skill (its system prompt and
//! recommended libraries), so the test exercises the actual selection,
//! prompt-construction, generation-wiring, and evaluation paths without network
//! access.

use async_trait::async_trait;

use paper2codes::benchmark::{
    run_benchmark, BenchmarkCase, BenchmarkManifest, GeneratedRepo, RepoGenerator,
};
use paper2codes::error::Result;
use paper2codes::skills::SkillRegistry;

/// A generator that emulates skill-guided code generation: it looks up the
/// chosen skill, builds the domain primer, and emits a module that imports the
/// skill's recommended libraries for the chosen language.
struct SkillGuidedGenerator {
    language: String,
}

#[async_trait]
impl RepoGenerator for SkillGuidedGenerator {
    async fn generate(&self, case: &BenchmarkCase) -> Result<GeneratedRepo> {
        let registry = SkillRegistry::with_builtins();
        let skill = registry.get(&case.id).ok_or_else(|| {
            paper2codes::error::Paper2CodesError::Validation(format!(
                "Unknown skill '{}'",
                case.id
            ))
        })?;

        // The domain primer would prefix the real coding prompt.
        let primer = skill.system_prompt(&self.language);
        assert!(primer.contains(&skill.name));

        let imports = skill
            .libraries_for(&self.language)
            .iter()
            .map(|lib| format!("import {}", lib))
            .collect::<Vec<_>>()
            .join("\n");

        let content = format!("# {} implementation\n{}\n\ndef main():\n    pass\n", skill.name, imports);
        Ok(GeneratedRepo {
            files: vec![("main.py".to_string(), content)],
        })
    }
}

#[tokio::test]
async fn user_selects_domain_uploads_paper_generates_and_evaluates() {
    // 1. The user browses available skills and selects one.
    let registry = SkillRegistry::with_builtins();
    assert!(registry.len() >= 8, "expected the full built-in skill catalog");

    let chosen = "quantum-computation";
    let skill = registry.get(chosen).expect("chosen skill exists");

    // The domain primer carries the right expertise + libraries for the language.
    let primer = skill.system_prompt("python");
    assert!(primer.to_lowercase().contains("qiskit"));

    // 2-4. Upload a paper and run skill-guided generation, evaluated by the harness.
    let manifest = BenchmarkManifest {
        name: Some("skill-e2e".to_string()),
        cases: vec![BenchmarkCase {
            id: chosen.to_string(),
            paper_path: "paper.txt".into(),
            reference_repo: None,
            rubric_path: None,
        }],
    };
    let generator = SkillGuidedGenerator {
        language: "python".to_string(),
    };

    let report = run_benchmark(&manifest, &generator, None).await;

    // 5. Evaluate: the scenario completed and produced a repository.
    assert_eq!(report.aggregate.case_count, 1);
    assert_eq!(report.aggregate.error_count, 0);
}

#[tokio::test]
async fn skill_guided_generation_scores_against_reference() {
    // A reference repo whose contents match a skill-guided generation should
    // score highly under reference-based evaluation.
    let dir = std::env::temp_dir().join(format!("p2c_skill_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let registry = SkillRegistry::with_builtins();
    let skill = registry.get("computational-finance").unwrap();
    let imports = skill
        .libraries_for("python")
        .iter()
        .map(|lib| format!("import {}", lib))
        .collect::<Vec<_>>()
        .join("\n");
    let reference_content =
        format!("# {} implementation\n{}\n\ndef main():\n    pass\n", skill.name, imports);
    std::fs::write(dir.join("main.py"), &reference_content).unwrap();

    let manifest = BenchmarkManifest {
        name: None,
        cases: vec![BenchmarkCase {
            id: "computational-finance".to_string(),
            paper_path: "paper.txt".into(),
            reference_repo: Some(dir.clone()),
            rubric_path: None,
        }],
    };
    let generator = SkillGuidedGenerator {
        language: "python".to_string(),
    };

    let report = run_benchmark(&manifest, &generator, None).await;
    let result = &report.cases[0];
    // Generated file matches the reference exactly → perfect file F1 + content.
    assert_eq!(result.file_f1, Some(1.0));
    assert_eq!(result.content_similarity, Some(1.0));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn auto_suggests_skill_from_paper_text() {
    // Part of the user journey: the engine can suggest a domain from the paper.
    let registry = SkillRegistry::with_builtins();
    let suggested = registry
        .match_skill("A variational quantum eigensolver ansatz for qubit Hamiltonians")
        .expect("a quantum skill should match");
    assert_eq!(suggested.id, "quantum-computation");

    let finance = registry
        .match_skill("Pricing exotic options via Monte Carlo and stochastic volatility")
        .expect("a finance skill should match");
    assert_eq!(finance.id, "computational-finance");
}
