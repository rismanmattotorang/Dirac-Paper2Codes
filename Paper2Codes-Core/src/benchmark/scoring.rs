//! Pure scoring functions for the benchmark harness.
//!
//! Two complementary families:
//! * **Reference-based** — compare a generated repository against an
//!   author-released reference (file-set precision/recall/F1 + content
//!   similarity). Requires ground truth.
//! * **Reference-free** — aggregate an LLM-judge rubric into a single score.
//!   No ground truth required.
//!
//! All functions here are deterministic and unit-tested; the LLM-judge call
//! that produces leaf scores lives behind a trait in [`super`].

use std::collections::HashSet;

use crate::retrieval::bm25;

/// Normalise a file path for matching: take the file name (basename),
/// lowercased. This makes matching robust to differing directory layouts
/// between the generated and reference repositories.
pub fn normalize_path(path: &str) -> String {
    let base = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim()
        .to_lowercase();
    base
}

/// File-set comparison metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSetScore {
    pub precision: f32,
    pub recall: f32,
    pub f1: f32,
    pub matched: usize,
    pub generated_total: usize,
    pub reference_total: usize,
}

/// Compare generated file paths against reference file paths by normalised name.
pub fn file_set_score(generated: &[String], reference: &[String]) -> FileSetScore {
    let gen: HashSet<String> = generated.iter().map(|p| normalize_path(p)).collect();
    let refer: HashSet<String> = reference.iter().map(|p| normalize_path(p)).collect();

    let matched = gen.intersection(&refer).count();
    let precision = ratio(matched, gen.len());
    let recall = ratio(matched, refer.len());
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    FileSetScore {
        precision,
        recall,
        f1,
        matched,
        generated_total: gen.len(),
        reference_total: refer.len(),
    }
}

fn ratio(num: usize, den: usize) -> f32 {
    if den == 0 {
        0.0
    } else {
        num as f32 / den as f32
    }
}

/// Token-set Jaccard similarity between two source texts, using the shared
/// retrieval tokenizer (identifiers, keywords; stop words removed).
pub fn identifier_jaccard(a: &str, b: &str) -> f32 {
    let ta: HashSet<String> = bm25::tokenize(a).into_iter().collect();
    let tb: HashSet<String> = bm25::tokenize(b).into_iter().collect();
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    ratio(inter, union)
}

/// Full reference-based score for a generated repo vs. a reference repo.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceScore {
    pub file: FileSetScore,
    /// Mean token-Jaccard over files present in both repos.
    pub content_similarity: f32,
    /// Combined headline score: `0.5 * file.f1 + 0.5 * content_similarity`.
    pub overall: f32,
}

/// Compute a [`ReferenceScore`] from `(path, content)` lists for the generated
/// and reference repositories. Content similarity is averaged over files whose
/// normalised names appear in both repos.
pub fn reference_score(
    generated: &[(String, String)],
    reference: &[(String, String)],
) -> ReferenceScore {
    let gen_paths: Vec<String> = generated.iter().map(|(p, _)| p.clone()).collect();
    let ref_paths: Vec<String> = reference.iter().map(|(p, _)| p.clone()).collect();
    let file = file_set_score(&gen_paths, &ref_paths);

    // Index reference content by normalised name for matched-file comparison.
    let ref_by_name: std::collections::HashMap<String, &String> = reference
        .iter()
        .map(|(p, c)| (normalize_path(p), c))
        .collect();

    let mut sims = Vec::new();
    for (path, content) in generated {
        if let Some(ref_content) = ref_by_name.get(&normalize_path(path)) {
            sims.push(identifier_jaccard(content, ref_content));
        }
    }
    let content_similarity = if sims.is_empty() {
        0.0
    } else {
        sims.iter().sum::<f32>() / sims.len() as f32
    };

    let overall = 0.5 * file.f1 + 0.5 * content_similarity;
    ReferenceScore {
        file,
        content_similarity,
        overall,
    }
}

/// Aggregate rubric leaf scores into a single `[0, 1]` value (mean, clamped).
pub fn aggregate_rubric_score(leaf_scores: &[f64]) -> f64 {
    if leaf_scores.is_empty() {
        return 0.0;
    }
    let mean = leaf_scores.iter().sum::<f64>() / leaf_scores.len() as f64;
    mean.clamp(0.0, 1.0)
}

/// Mean of a slice of `f32`, or 0.0 when empty.
pub fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_takes_basename_lowercased() {
        assert_eq!(normalize_path("src/Model/Net.py"), "net.py");
        assert_eq!(normalize_path("Attention.PY"), "attention.py");
    }

    #[test]
    fn file_set_score_precision_recall_f1() {
        let gen = vec!["a.py".to_string(), "b.py".to_string(), "extra.py".to_string()];
        let refer = vec!["a.py".to_string(), "b.py".to_string(), "c.py".to_string()];
        let s = file_set_score(&gen, &refer);
        assert_eq!(s.matched, 2);
        assert!((s.precision - 2.0 / 3.0).abs() < 1e-6);
        assert!((s.recall - 2.0 / 3.0).abs() < 1e-6);
        assert!((s.f1 - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn file_set_score_empty_is_zero_not_nan() {
        let s = file_set_score(&[], &[]);
        assert_eq!(s.f1, 0.0);
        assert!(s.f1.is_finite());
    }

    #[test]
    fn identifier_jaccard_basic() {
        assert!((identifier_jaccard("def attention(query)", "def attention(query)") - 1.0).abs() < 1e-6);
        assert_eq!(identifier_jaccard("alpha beta", "gamma delta"), 0.0);
        let both_empty = identifier_jaccard("the a an", "is to of"); // all stop words
        assert_eq!(both_empty, 1.0);
    }

    #[test]
    fn reference_score_combines_files_and_content() {
        let generated = vec![
            ("model/net.py".to_string(), "def forward(x): return relu(x)".to_string()),
            ("util.py".to_string(), "def helper(): pass".to_string()),
        ];
        let reference = vec![
            ("net.py".to_string(), "def forward(x): return relu(x)".to_string()),
            ("missing.py".to_string(), "def other(): pass".to_string()),
        ];
        let s = reference_score(&generated, &reference);
        assert_eq!(s.file.matched, 1); // net.py matches by basename
        // matched file has identical content -> similarity 1.0
        assert!((s.content_similarity - 1.0).abs() < 1e-6);
        assert!(s.overall > 0.0 && s.overall <= 1.0);
    }

    #[test]
    fn aggregate_rubric_clamps_and_means() {
        assert!((aggregate_rubric_score(&[0.5, 1.0]) - 0.75).abs() < 1e-9);
        assert_eq!(aggregate_rubric_score(&[]), 0.0);
        assert_eq!(aggregate_rubric_score(&[2.0, 2.0]), 1.0); // clamped
    }
}
