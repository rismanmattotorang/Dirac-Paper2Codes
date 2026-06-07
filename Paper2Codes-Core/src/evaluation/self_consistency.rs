//! Self-consistency / verification-guided selection for code generation.
//!
//! A single LLM sample is a coin flip on hard implementation tasks. Sampling
//! several candidates and selecting the best — by verification outcome first,
//! then judge score, then majority agreement — is a well-established way to lift
//! correctness (self-consistency; ALGO-style verifier-guided selection).
//!
//! This module provides:
//! * [`Candidate`] — a scored candidate (verification metrics + optional judge
//!   score + a content signature for agreement voting),
//! * [`select_best`] — the pure, deterministic selection policy, and
//! * [`best_of_n`] — an async driver that samples `n` candidates from a
//!   [`CandidateGenerator`], scores each with a caller-supplied async scorer, and
//!   returns the winner.
//!
//! The LLM-dependent sampling sits behind the [`CandidateGenerator`] trait so
//! the whole loop is unit-testable with a mock.

use async_trait::async_trait;

use crate::error::Result;

/// A scored code-generation candidate.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// Whether verification passed (no critical/error issues).
    pub passed: bool,
    /// Failed checks reported by verification.
    pub failed_checks: usize,
    /// Total checks attempted (0 if verification did not run).
    pub total_checks: usize,
    /// Optional LLM-judge score in `[0, 1]`.
    pub judge_score: Option<f32>,
    /// Normalised content signature used for majority/agreement voting.
    pub signature: String,
}

impl Candidate {
    /// Fraction of checks that passed, in `[0, 1]`. With no checks, a passing
    /// candidate scores 1.0 and a failing one 0.0.
    pub fn pass_rate(&self) -> f32 {
        if self.total_checks == 0 {
            if self.passed {
                1.0
            } else {
                0.0
            }
        } else {
            let passed = self.total_checks.saturating_sub(self.failed_checks);
            passed as f32 / self.total_checks as f32
        }
    }
}

/// Count how many candidates share `signature` (agreement / majority voting).
fn agreement_count(candidates: &[Candidate], signature: &str) -> usize {
    candidates
        .iter()
        .filter(|c| c.signature == signature)
        .count()
}

/// Select the best candidate index using a deterministic lexicographic policy:
///
/// 1. verification passed (true before false),
/// 2. higher pass-rate,
/// 3. higher judge score (treating absent as 0),
/// 4. larger agreement cluster (majority vote),
/// 5. lowest index (stable tie-break).
pub fn select_best(candidates: &[Candidate]) -> Option<usize> {
    if candidates.is_empty() {
        return None;
    }

    let mut best_idx = 0usize;
    let mut best_key = ranking_key(candidates, 0);

    for idx in 1..candidates.len() {
        let key = ranking_key(candidates, idx);
        if key > best_key {
            best_key = key;
            best_idx = idx;
        }
    }
    Some(best_idx)
}

/// Build a comparable ranking key for candidate `idx`. Higher is better; the
/// final negative index makes lower indices win ties.
fn ranking_key(candidates: &[Candidate], idx: usize) -> (u8, OrderedF32, OrderedF32, usize, i64) {
    let c = &candidates[idx];
    (
        c.passed as u8,
        OrderedF32(c.pass_rate()),
        OrderedF32(c.judge_score.unwrap_or(0.0)),
        agreement_count(candidates, &c.signature),
        -(idx as i64),
    )
}

/// Total-orderable f32 wrapper (NaN treated as the smallest value) so ranking
/// keys can derive `Ord`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedF32(f32);
impl Eq for OrderedF32 {}
impl PartialOrd for OrderedF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Generates a code candidate for a given attempt index.
#[async_trait]
pub trait CandidateGenerator: Send + Sync {
    /// Produce the raw generated content for `attempt` (0-based).
    async fn generate(&self, attempt: usize) -> Result<String>;
}

/// Sample `n` candidates, score each with `scorer`, and return the index and
/// content of the best one. `n` is clamped to at least 1. Generation failures
/// are skipped; if every attempt fails, the last error is returned.
pub async fn best_of_n<G, S, Fut>(
    generator: &G,
    n: usize,
    scorer: S,
) -> Result<(usize, String)>
where
    G: CandidateGenerator,
    S: Fn(usize, &str) -> Fut,
    Fut: std::future::Future<Output = Candidate>,
{
    let n = n.max(1);
    let mut contents: Vec<String> = Vec::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut last_err = None;

    for attempt in 0..n {
        match generator.generate(attempt).await {
            Ok(content) => {
                let candidate = scorer(attempt, &content).await;
                contents.push(content);
                candidates.push(candidate);
            }
            Err(e) => {
                tracing::warn!("Candidate generation attempt {} failed: {}", attempt, e);
                last_err = Some(e);
            }
        }
    }

    match select_best(&candidates) {
        Some(idx) => Ok((idx, contents[idx].clone())),
        None => Err(last_err.unwrap_or_else(|| {
            crate::error::Paper2CodesError::Validation(
                "No code candidates were generated".to_string(),
            )
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn cand(passed: bool, failed: usize, total: usize, judge: Option<f32>, sig: &str) -> Candidate {
        Candidate {
            passed,
            failed_checks: failed,
            total_checks: total,
            judge_score: judge,
            signature: sig.to_string(),
        }
    }

    #[test]
    fn prefers_passing_candidate() {
        let cands = vec![
            cand(false, 1, 4, Some(0.9), "a"),
            cand(true, 0, 4, Some(0.1), "b"),
        ];
        assert_eq!(select_best(&cands), Some(1));
    }

    #[test]
    fn breaks_ties_by_pass_rate_then_judge() {
        let cands = vec![
            cand(true, 2, 4, Some(0.5), "a"), // pass_rate 0.5
            cand(true, 1, 4, Some(0.2), "b"), // pass_rate 0.75 -> wins
        ];
        assert_eq!(select_best(&cands), Some(1));

        let cands = vec![
            cand(true, 1, 4, Some(0.2), "a"),
            cand(true, 1, 4, Some(0.8), "b"), // same pass_rate, higher judge -> wins
        ];
        assert_eq!(select_best(&cands), Some(1));
    }

    #[test]
    fn uses_majority_agreement_as_tiebreak() {
        // All equal on passed/pass_rate/judge; "x" appears twice -> its first
        // occurrence wins on agreement count.
        let cands = vec![
            cand(true, 0, 0, None, "x"),
            cand(true, 0, 0, None, "y"),
            cand(true, 0, 0, None, "x"),
        ];
        assert_eq!(select_best(&cands), Some(0));
    }

    #[test]
    fn empty_is_none() {
        assert_eq!(select_best(&[]), None);
    }

    struct MockGen;
    #[async_trait]
    impl CandidateGenerator for MockGen {
        async fn generate(&self, attempt: usize) -> Result<String> {
            Ok(format!("candidate-{}", attempt))
        }
    }

    #[tokio::test]
    async fn best_of_n_selects_highest_scoring() {
        let calls = AtomicUsize::new(0);
        let (idx, content) = best_of_n(&MockGen, 3, |attempt, _content| {
            calls.fetch_add(1, Ordering::SeqCst);
            async move {
                // attempt 2 passes verification; others fail.
                cand(attempt == 2, if attempt == 2 { 0 } else { 1 }, 2, None, "sig")
            }
        })
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(idx, 2);
        assert_eq!(content, "candidate-2");
    }
}
