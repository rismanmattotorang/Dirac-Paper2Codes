//! LLM-as-judge rubric grading (reference-free benchmark scoring).
//!
//! Complements reference-based scoring (which needs an author repo) by asking
//! an LLM to rate, in `[0, 1]`, how faithfully a generated repository implements
//! the source paper. The numeric-parsing logic is pure and unit-tested; the LLM
//! call is isolated behind [`super::RubricGrader`].

use std::sync::Arc;

use async_trait::async_trait;

use super::{BenchmarkCase, GeneratedRepo, RubricGrader};
use crate::error::Result;
use crate::llm::client::LLMClient;
use crate::llm::{LLMRequest, Message, MessageRole};

/// Parse an LLM judge response into a score in `[0, 1]`.
///
/// Tolerant of formats: a bare `0.83`, a percentage `83%` / `83`, or a number
/// embedded in prose ("Score: 0.83 because..."). Values > 1 are treated as
/// percentages; the result is clamped to `[0, 1]`. Returns `0.0` if no number
/// is found.
pub fn parse_rubric_score(text: &str) -> f64 {
    let mut number = String::new();
    let mut found: Option<f64> = None;
    let mut percent = false;

    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_digit() || (c == '.' && !number.contains('.')) {
            number.push(c);
        } else if !number.is_empty() {
            if let Ok(v) = number.parse::<f64>() {
                found = Some(v);
                percent = c == '%' || chars.get(i).is_some_and(|c| *c == '%');
                break;
            }
            number.clear();
        }
    }
    if found.is_none() && !number.is_empty() {
        found = number.parse::<f64>().ok();
    }

    let mut score = match found {
        Some(v) => v,
        None => return 0.0,
    };
    if percent || score > 1.0 {
        score /= 100.0;
    }
    score.clamp(0.0, 1.0)
}

/// LLM-backed rubric grader.
pub struct LlmRubricGrader {
    client: Arc<dyn LLMClient>,
    /// Max characters of generated content to include per file in the prompt.
    max_file_chars: usize,
}

impl LlmRubricGrader {
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self {
            client,
            max_file_chars: 2000,
        }
    }

    fn build_prompt(&self, case: &BenchmarkCase, repo: &GeneratedRepo) -> String {
        let paper = std::fs::read_to_string(&case.paper_path).unwrap_or_default();
        let rubric = case
            .rubric_path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();

        let mut code = String::new();
        for (path, content) in &repo.files {
            let truncated: String = content.chars().take(self.max_file_chars).collect();
            code.push_str(&format!("\n--- {} ---\n{}\n", path, truncated));
        }

        let mut prompt = String::new();
        if !rubric.trim().is_empty() {
            prompt.push_str(&format!("Grading rubric:\n{}\n\n", rubric));
        }
        prompt.push_str(&format!(
            "Source paper:\n{}\n\nGenerated repository:\n{}\n",
            paper.chars().take(8000).collect::<String>(),
            code
        ));
        prompt
    }
}

#[async_trait]
impl RubricGrader for LlmRubricGrader {
    async fn grade(&self, case: &BenchmarkCase, repo: &GeneratedRepo) -> Result<f64> {
        let system = "You are a strict but fair code reviewer evaluating whether a \
            generated repository faithfully implements a scientific paper. Consider \
            correctness, completeness, and use of the right methods/equations. \
            Respond with ONLY a single number between 0 and 1 (e.g. 0.82) — your \
            overall faithfulness score.";

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: system.to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: self.build_prompt(case, repo),
                },
            ],
            self.client.model(),
        )
        .with_temperature(0.0)
        .with_max_tokens(16);

        let response = self.client.complete(request).await?;
        Ok(parse_rubric_score(&response.content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_fraction() {
        assert!((parse_rubric_score("0.82") - 0.82).abs() < 1e-9);
        assert!((parse_rubric_score("Score: 0.5 — partial") - 0.5).abs() < 1e-9);
    }

    #[test]
    fn parses_percentages_and_bare_integers() {
        assert!((parse_rubric_score("83%") - 0.83).abs() < 1e-9);
        assert!((parse_rubric_score("85") - 0.85).abs() < 1e-9);
        assert!((parse_rubric_score("1") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clamps_and_handles_missing() {
        assert_eq!(parse_rubric_score("150"), 1.0);
        assert_eq!(parse_rubric_score("no number here"), 0.0);
        assert_eq!(parse_rubric_score(""), 0.0);
    }
}
