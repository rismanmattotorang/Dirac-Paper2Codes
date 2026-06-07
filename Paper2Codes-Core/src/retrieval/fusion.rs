//! Reciprocal Rank Fusion (RRF) for combining multiple ranked retrieval lists.
//!
//! Dense (embedding) and sparse (BM25) retrievers fail in orthogonal ways and
//! produce scores on incompatible scales — cosine similarity lives in `[-1, 1]`
//! while BM25 is an unbounded positive float. Naively averaging those scores
//! (as the previous CPR implementation effectively did) lets whichever scale is
//! larger dominate the ranking.
//!
//! RRF sidesteps this entirely by fusing *ranks* rather than scores: a document
//! at rank `r` (0-based) in a list contributes `1 / (k + r + 1)` to its fused
//! score, summed across all lists. It needs no score normalisation, is
//! unsupervised, and is the de-facto production standard (Elasticsearch,
//! Qdrant, Weaviate all implement it). The constant `k` (commonly 60) damps the
//! influence of very high ranks.
//!
//! Reference: Cormack, Clarke & Büttcher, "Reciprocal Rank Fusion outperforms
//! Condorcet and individual Rank Learning Methods" (SIGIR 2009).

/// Default RRF damping constant, per the original paper and common practice.
pub const DEFAULT_RRF_K: f32 = 60.0;

/// Fuse several ranked lists of ids into a single ranking.
///
/// Each input list is ordered best-first. The returned list is `(id, score)`
/// sorted by descending fused score; every id that appears in at least one
/// input list is included exactly once.
pub fn reciprocal_rank_fusion(rankings: &[Vec<String>], k: f32) -> Vec<(String, f32)> {
    use std::collections::HashMap;

    let mut fused: HashMap<String, f32> = HashMap::new();
    // Preserve first-seen order for deterministic tie-breaking.
    let mut order: Vec<String> = Vec::new();

    for ranking in rankings {
        for (rank, id) in ranking.iter().enumerate() {
            let contribution = 1.0 / (k + rank as f32 + 1.0);
            let entry = fused.entry(id.clone()).or_insert_with(|| {
                order.push(id.clone());
                0.0
            });
            *entry += contribution;
        }
    }

    let mut result: Vec<(String, f32)> = order
        .into_iter()
        .map(|id| {
            let score = fused[&id];
            (id, score)
        })
        .collect();

    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Fuse ranked lists and return scores normalised to `[0, 1]` (divided by the
/// top fused score), as a lookup map. Convenient when the fused relevance needs
/// to be combined with other `[0, 1]` signals (structural boosts, de-boosts).
pub fn fused_relevance_map(
    rankings: &[Vec<String>],
    k: f32,
) -> std::collections::HashMap<String, f32> {
    let fused = reciprocal_rank_fusion(rankings, k);
    let max = fused.first().map(|(_, s)| *s).unwrap_or(0.0);
    fused
        .into_iter()
        .map(|(id, score)| {
            let norm = if max > 0.0 { score / max } else { 0.0 };
            (id, norm)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn agreement_across_lists_ranks_first() {
        // "b" is ranked highly by both retrievers; it should win the fusion.
        let dense = ids(&["a", "b", "c"]);
        let sparse = ids(&["b", "d", "a"]);
        let fused = reciprocal_rank_fusion(&[dense, sparse], DEFAULT_RRF_K);
        assert_eq!(fused[0].0, "b");
    }

    #[test]
    fn handles_disjoint_and_single_lists() {
        let dense = ids(&["a", "b"]);
        let sparse = ids(&["c", "d"]);
        let fused = reciprocal_rank_fusion(&[dense, sparse], DEFAULT_RRF_K);
        // All four ids present; the two rank-0 entries (a, c) tie at the top.
        assert_eq!(fused.len(), 4);
        let top_two: Vec<&str> = fused.iter().take(2).map(|(id, _)| id.as_str()).collect();
        assert!(top_two.contains(&"a"));
        assert!(top_two.contains(&"c"));
    }

    #[test]
    fn normalised_map_top_is_one() {
        let dense = ids(&["a", "b", "c"]);
        let sparse = ids(&["a", "c", "b"]);
        let map = fused_relevance_map(&[dense, sparse], DEFAULT_RRF_K);
        // "a" is rank 0 in both, so it has the max fused score -> normalised 1.0.
        assert!((map["a"] - 1.0).abs() < 1e-6);
        assert!(map["b"] <= 1.0 && map["b"] >= 0.0);
    }

    #[test]
    fn empty_input_yields_empty_output() {
        let fused = reciprocal_rank_fusion(&[], DEFAULT_RRF_K);
        assert!(fused.is_empty());
    }
}
