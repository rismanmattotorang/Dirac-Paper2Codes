//! Corrective Retrieval (CRAG-style) confidence grading.
//!
//! Single-shot retrieval can silently return weak context, which downstream
//! agents then hallucinate around. Following Corrective RAG (Yan et al., 2024),
//! we grade the *confidence* of a retrieval result from the relevance scores of
//! its top hits and take a corrective action when confidence is low.
//!
//! The grade drives a deterministic **knowledge-expansion** action: when the
//! retriever is unsure, we widen selection to hand the agent more candidate
//! context rather than a thin, possibly-wrong top-k. The grading and the
//! expansion factor are pure functions, unit-tested below; the integration
//! lives in [`crate::retrieval::DefaultCPREngine::retrieve`].

/// Confidence grade for a retrieval result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalGrade {
    /// Strong top hit — use the result as-is.
    Correct,
    /// Middling confidence — expand context moderately.
    Ambiguous,
    /// Weak top hit — expand context aggressively.
    Incorrect,
}

/// Score thresholds separating the three confidence bands. Scores are the
/// normalised `[0, 1]` relevance values produced by the hybrid scorer.
#[derive(Debug, Clone, Copy)]
pub struct CragThresholds {
    pub upper: f32,
    pub lower: f32,
}

impl Default for CragThresholds {
    fn default() -> Self {
        Self {
            upper: 0.6,
            lower: 0.3,
        }
    }
}

/// Grade a retrieval result from its (already sorted, descending) top scores.
pub fn grade(top_scores: &[f32], thresholds: &CragThresholds) -> RetrievalGrade {
    let max = top_scores
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    if !max.is_finite() {
        return RetrievalGrade::Incorrect;
    }
    if max >= thresholds.upper {
        RetrievalGrade::Correct
    } else if max < thresholds.lower {
        RetrievalGrade::Incorrect
    } else {
        RetrievalGrade::Ambiguous
    }
}

/// Effective number of contexts to select given the base `k` and a grade.
///
/// Correct → `k`; Ambiguous → `k + ⌈k/2⌉`; Incorrect → `2k` (at least `k + 1`
/// when a correction is warranted). Always returns at least 1.
pub fn expanded_k(base_k: usize, grade: RetrievalGrade) -> usize {
    let expanded = match grade {
        RetrievalGrade::Correct => base_k,
        RetrievalGrade::Ambiguous => base_k + base_k.div_ceil(2).max(1),
        RetrievalGrade::Incorrect => (base_k * 2).max(base_k + 1),
    };
    expanded.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_by_top_score() {
        let t = CragThresholds::default();
        assert_eq!(grade(&[0.9, 0.2], &t), RetrievalGrade::Correct);
        assert_eq!(grade(&[0.45, 0.4], &t), RetrievalGrade::Ambiguous);
        assert_eq!(grade(&[0.1, 0.05], &t), RetrievalGrade::Incorrect);
        assert_eq!(grade(&[], &t), RetrievalGrade::Incorrect);
    }

    #[test]
    fn expansion_grows_with_uncertainty() {
        assert_eq!(expanded_k(4, RetrievalGrade::Correct), 4);
        assert_eq!(expanded_k(4, RetrievalGrade::Ambiguous), 6);
        assert_eq!(expanded_k(4, RetrievalGrade::Incorrect), 8);
        // Never collapses below 1, and Incorrect always expands.
        assert_eq!(expanded_k(0, RetrievalGrade::Correct), 1);
        assert_eq!(expanded_k(1, RetrievalGrade::Incorrect), 2);
    }
}
