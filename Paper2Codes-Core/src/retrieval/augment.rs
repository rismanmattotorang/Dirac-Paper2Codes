//! Phase 3 retrieval augmentation: HyDE query transformation and learned
//! reranking.
//!
//! Both are *optional* enhancements layered on top of the Phase 1/2 hybrid
//! retriever. They require an LLM, so the engine falls back cleanly to BM25 +
//! RRF + structural reranking when no LLM client is wired.
//!
//! * **HyDE** (Hypothetical Document Embeddings): instead of embedding the bare
//!   task description, we ask an LLM to write a short hypothetical
//!   implementation sketch — text that looks like the passages we want to find —
//!   and embed *that*, bridging the query↔document vocabulary gap. The
//!   hypothetical embedding is blended with the original-query embedding so a
//!   poor hypothesis cannot fully derail retrieval.
//! * **Learned reranking**: the fused top-N candidates are re-ordered by an LLM
//!   acting as a cross-encoder-style reranker, improving top-of-list precision
//!   before MMR diversity selection.
//!
//! The deterministic pieces — embedding blending and parsing the reranker's
//! response into an ordering — are pure functions and unit-tested here; the LLM
//! calls themselves are isolated behind the [`QueryExpander`] / [`Reranker`]
//! traits so they can be mocked.

use async_trait::async_trait;
use std::sync::Arc;

use crate::error::Result;
use crate::llm::client::LLMClient;
use crate::llm::{LLMRequest, Message, MessageRole};
use crate::types::{RetrievedContext, Task};

/// Produces a hypothetical document for a task to improve dense retrieval (HyDE).
#[async_trait]
pub trait QueryExpander: Send + Sync {
    /// Return a hypothetical passage for `task`, or `None` to skip expansion.
    async fn expand(&self, task: &Task) -> Result<Option<String>>;
}

/// Re-orders candidate contexts for a task (cross-encoder-style reranking).
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Return a new ordering as indices into `candidates`, best first. The
    /// returned ordering should cover all candidate indices exactly once.
    async fn rerank(&self, task: &Task, candidates: &[RetrievedContext]) -> Result<Vec<usize>>;
}

/// Blend two embeddings: `alpha * a + (1 - alpha) * b`, then L2-normalise.
///
/// Used to combine the original-query embedding (`a`) with the HyDE
/// hypothetical-document embedding (`b`). On a dimension mismatch we defensively
/// return the original-query embedding unchanged.
pub fn combine_embeddings(a: &[f32], b: &[f32], alpha: f32) -> Vec<f32> {
    if a.len() != b.len() || a.is_empty() {
        return a.to_vec();
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let mut combined: Vec<f32> = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| alpha * x + (1.0 - alpha) * y)
        .collect();

    let norm: f32 = combined.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm.is_finite() && norm > 0.0 {
        for v in combined.iter_mut() {
            *v /= norm;
        }
    }
    combined
}

/// Parse an LLM reranking response into a 0-based ordering over `n` candidates.
///
/// The reranker is prompted to return candidate numbers (1-based) from most to
/// least relevant, e.g. `"3, 1, 2"`. This parser is deliberately forgiving: it
/// extracts integers in any surrounding text, converts them to 0-based indices,
/// drops out-of-range and duplicate values, and appends any candidate indices
/// the model omitted (in their original order) so the result is always a
/// complete permutation of `0..n`.
pub fn parse_rerank_order(response: &str, n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let mut seen = vec![false; n];

    let mut digits = String::new();
    let flush = |digits: &mut String, order: &mut Vec<usize>, seen: &mut [bool]| {
        if digits.is_empty() {
            return;
        }
        if let Ok(one_based) = digits.parse::<usize>() {
            if one_based >= 1 && one_based <= n {
                let idx = one_based - 1;
                if !seen[idx] {
                    seen[idx] = true;
                    order.push(idx);
                }
            }
        }
        digits.clear();
    };

    for ch in response.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            flush(&mut digits, &mut order, &mut seen);
        }
    }
    flush(&mut digits, &mut order, &mut seen);

    // Append any indices the model omitted, preserving original order.
    for (idx, was_seen) in seen.iter().enumerate() {
        if !was_seen {
            order.push(idx);
        }
    }

    order
}

/// Truncate text to at most `max_chars` characters (on a char boundary).
fn truncate(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut s: String = trimmed.chars().take(max_chars).collect();
        s.push('…');
        s
    }
}

/// LLM-backed HyDE query expander.
pub struct LlmQueryExpander {
    client: Arc<dyn LLMClient>,
}

impl LlmQueryExpander {
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl QueryExpander for LlmQueryExpander {
    async fn expand(&self, task: &Task) -> Result<Option<String>> {
        let system = "You help retrieve passages from a scientific paper that are \
            useful for implementing a software task. Given a task, write a concise \
            (3-5 sentence) hypothetical excerpt — using the technical terminology, \
            equation names, and algorithmic vocabulary that such a paper passage \
            would contain. Output only the excerpt, no preamble.";
        let prompt = format!("Task: {}", task.description);

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: system.to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            self.client.model(),
        )
        .with_temperature(0.3)
        .with_max_tokens(256);

        let response = self.client.complete(request).await?;
        let content = response.content.trim().to_string();
        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }
}

/// LLM-backed cross-encoder-style reranker.
pub struct LlmReranker {
    client: Arc<dyn LLMClient>,
}

impl LlmReranker {
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Reranker for LlmReranker {
    async fn rerank(&self, task: &Task, candidates: &[RetrievedContext]) -> Result<Vec<usize>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let mut passages = String::new();
        for (i, ctx) in candidates.iter().enumerate() {
            passages.push_str(&format!(
                "{}. {}\n",
                i + 1,
                truncate(&ctx.segment.content, 300)
            ));
        }

        let system = "You are a reranking model. Given a task and a numbered list \
            of candidate passages, order the passages by how directly useful they \
            are for accomplishing the task. Respond with ONLY a comma-separated \
            list of passage numbers, most relevant first (e.g. \"3, 1, 4, 2\"). \
            Include every number exactly once.";
        let prompt = format!(
            "Task: {}\n\nCandidate passages:\n{}",
            task.description, passages
        );

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: system.to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            self.client.model(),
        )
        .with_temperature(0.0)
        .with_max_tokens(256);

        let response = self.client.complete(request).await?;
        Ok(parse_rerank_order(&response.content, candidates.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_embeddings_blends_and_normalises() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let c = combine_embeddings(&a, &b, 0.5);
        // Equal blend then normalise -> components equal, unit length.
        assert!((c[0] - c[1]).abs() < 1e-6);
        let norm: f32 = c.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn combine_embeddings_alpha_one_returns_query_direction() {
        let a = vec![3.0, 4.0]; // norm 5
        let b = vec![0.0, 1.0];
        let c = combine_embeddings(&a, &b, 1.0);
        // alpha=1 -> direction of a, normalised.
        assert!((c[0] - 0.6).abs() < 1e-6);
        assert!((c[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn combine_embeddings_dimension_mismatch_falls_back() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0];
        assert_eq!(combine_embeddings(&a, &b, 0.5), a);
    }

    #[test]
    fn parse_rerank_order_basic() {
        assert_eq!(parse_rerank_order("3, 1, 2", 3), vec![2, 0, 1]);
    }

    #[test]
    fn parse_rerank_order_tolerates_noise_and_brackets() {
        assert_eq!(parse_rerank_order("Order: [2, 1, 3].", 3), vec![1, 0, 2]);
    }

    #[test]
    fn parse_rerank_order_appends_missing_and_drops_out_of_range() {
        // "5" is out of range and dropped; missing 2,3 appended in order.
        assert_eq!(parse_rerank_order("1, 5", 3), vec![0, 1, 2]);
    }

    #[test]
    fn parse_rerank_order_dedups() {
        assert_eq!(parse_rerank_order("2, 2, 1", 3), vec![1, 0, 2]);
    }

    #[test]
    fn parse_rerank_order_empty_response_is_identity() {
        assert_eq!(parse_rerank_order("no numbers here", 3), vec![0, 1, 2]);
    }
}
