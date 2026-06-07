//! Hierarchical rubric evaluation system
use crate::error::Result;
use crate::llm::{LLMClient, LLMRequest, Message, MessageRole};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

/// Requirement types for evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementType {
    CodeDevelopment,
    Execution,
    ResultMatch,
}

impl std::fmt::Display for RequirementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CodeDevelopment => write!(f, "CodeDevelopment"),
            Self::Execution => write!(f, "Execution"),
            Self::ResultMatch => write!(f, "ResultMatch"),
        }
    }
}

/// Grading result for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGradingResult {
    pub node_id: String,
    pub score: f64,
    pub explanation: String,
    pub requirement_type: RequirementType,
    pub is_leaf: bool,
}

/// Overall grading result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradingResult {
    pub score: f64,
    pub node_results: Vec<NodeGradingResult>,
    pub metadata: GradingMetadata,
}

/// Grading metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradingMetadata {
    pub model: String,
    pub total_tokens: usize,
    pub api_calls: usize,
    pub time_taken: std::time::Duration,
}

/// Rubric node structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricNode {
    pub id: String,
    pub requirements: String,
    pub requirement_type: Option<RequirementType>,
    pub children: Vec<RubricNode>,
}

impl RubricNode {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn leaf_nodes(&self) -> Vec<&RubricNode> {
        let mut leaves = Vec::new();
        if self.is_leaf() {
            leaves.push(self);
        } else {
            for child in &self.children {
                leaves.extend(child.leaf_nodes());
            }
        }
        leaves
    }
}

/// Simple judge for hierarchical rubric evaluation
pub struct SimpleJudge {
    llm_client: Arc<dyn LLMClient>,
    max_files: usize,
}

impl SimpleJudge {
    pub fn new(llm_client: Arc<dyn LLMClient>, max_files: usize) -> Self {
        Self {
            llm_client,
            max_files,
        }
    }

    /// Grade a submission using a hierarchical rubric
    pub async fn grade(
        &self,
        submission_path: &Path,
        rubric: &RubricNode,
        paper_content: &str,
    ) -> Result<GradingResult> {
        let start_time = std::time::Instant::now();
        let leaf_nodes = rubric.leaf_nodes();

        info!("Grading {} leaf nodes", leaf_nodes.len());

        let mut node_results = Vec::new();
        let mut leaf_scores = HashMap::new();
        let mut total_tokens = 0usize;

        for (i, node) in leaf_nodes.iter().enumerate() {
            info!("Grading node {}/{}: {}", i + 1, leaf_nodes.len(), node.id);

            match self
                .grade_leaf_node_with_tokens(node, submission_path, paper_content, rubric)
                .await
            {
                Ok((result, tokens)) => {
                    leaf_scores.insert(result.node_id.clone(), result.score);
                    node_results.push(result);
                    total_tokens += tokens as usize;
                }
                Err(e) => {
                    error!("Failed to grade node {}: {}", node.id, e);
                    node_results.push(NodeGradingResult {
                        node_id: node.id.clone(),
                        score: 0.0,
                        explanation: format!("Grading failed: {}", e),
                        requirement_type: node
                            .requirement_type
                            .unwrap_or(RequirementType::CodeDevelopment),
                        is_leaf: true,
                    });
                }
            }
        }

        // Calculate overall score (simple average for now)
        let overall_score = if !leaf_scores.is_empty() {
            leaf_scores.values().sum::<f64>() / leaf_scores.len() as f64
        } else {
            0.0
        };

        let time_taken = start_time.elapsed();

        Ok(GradingResult {
            score: overall_score,
            node_results,
            metadata: GradingMetadata {
                model: self.llm_client.model(),
                total_tokens,
                api_calls: leaf_nodes.len(),
                time_taken,
            },
        })
    }

    async fn grade_leaf_node_with_tokens(
        &self,
        node: &RubricNode,
        submission_path: &Path,
        paper_content: &str,
        _rubric: &RubricNode,
    ) -> Result<(NodeGradingResult, usize)> {
        let requirement_type = node
            .requirement_type
            .unwrap_or(RequirementType::CodeDevelopment);

        // Get relevant files
        let mut relevant_files = self
            .get_relevant_files(submission_path, requirement_type)
            .await?;

        // Rank files by relevance
        relevant_files = crate::evaluation::file_ranking::rank_files_by_relevance(
            &relevant_files,
            &node.requirements,
            self.max_files,
        )
        .await?;

        let files_to_use = relevant_files
            .iter()
            .take(self.max_files)
            .collect::<Vec<_>>();

        // Build file contents
        let mut files_content = String::new();
        for file_path in &files_to_use {
            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                files_content.push_str(&format!(
                    "=== {} ===\n{}\n\n",
                    file_path.file_name().unwrap_or_default().to_string_lossy(),
                    content
                ));
            }
        }

        // Build prompt with better structure
        let task_question = match requirement_type {
            RequirementType::CodeDevelopment => {
                "Does the agent's source code contain a correct implementation of this?"
            }
            RequirementType::Execution => {
                "Does running the reproduce.sh script lead to this being successfully executed?"
            }
            RequirementType::ResultMatch => {
                "Does the outcome of the reproduction agree with these results?"
            }
        };

        // Limit paper content to prevent token overflow (keep more context)
        let paper_snippet = if paper_content.len() > 8000 {
            format!(
                "{}...\n[Content truncated for length]",
                &paper_content[..8000]
            )
        } else {
            paper_content.to_string()
        };

        let prompt = format!(
            r#"The paper is below:
{}

Here are the most relevant files from the submission:
<files>
{}
</files>

The criterion you are grading:
<criterion>
{}
</criterion>

The criterion is of type {} - {}

Evaluate the submission above for the given criterion.
Present your answer in 3 parts:

# Expectations
Describe what you expect correct resolution to look like.

# Reality
Explore the files and comment on how they compare to your expectations.

# Score
Give a score of either 0 or 1. Return "Score: 0" or "Score: 1" on its own line."#,
            &paper_snippet, files_content, node.requirements, requirement_type, task_question
        );

        let request = LLMRequest {
            model: self.llm_client.model(),
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: "You are an expert evaluator judging attempts to reproduce ML research papers.".to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            temperature: 0.0,
            max_tokens: Some(2048),
            stream: false,
        };

        let response = self.llm_client.complete(request).await?;
        let tokens_used = response.tokens_used.unwrap_or(0) as usize;
        let (score, explanation) = parse_judge_response(&response.content).map_err(|e| {
            crate::error::Paper2CodesError::Validation(format!(
                "Failed to parse judge response for node {}: {}",
                node.id, e
            ))
        })?;

        Ok((
            NodeGradingResult {
                node_id: node.id.clone(),
                score,
                explanation,
                requirement_type,
                is_leaf: true,
            },
            tokens_used,
        ))
    }

    async fn get_relevant_files(
        &self,
        submission_path: &Path,
        requirement_type: RequirementType,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        match requirement_type {
            RequirementType::CodeDevelopment => {
                let code_extensions = &[
                    ".py", ".rs", ".js", ".ts", ".java", ".c", ".cpp", ".h", ".hpp", ".sh",
                    ".bash", ".go", ".rb", ".php", ".pl", ".R", ".jl",
                ];

                for entry in walkdir::WalkDir::new(submission_path) {
                    let entry = entry.map_err(|e| {
                        crate::error::Paper2CodesError::Validation(format!("WalkDir error: {}", e))
                    })?;
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            let ext_str = format!(".{}", ext.to_string_lossy());
                            if code_extensions.contains(&ext_str.as_str()) {
                                files.push(entry.path().to_path_buf());
                            }
                        }
                    }
                }
            }
            RequirementType::Execution => {
                files.push(submission_path.join("reproduce.sh"));
                files.push(submission_path.join("reproduce.log"));
            }
            RequirementType::ResultMatch => {
                files.push(submission_path.join("reproduce.sh"));
                files.push(submission_path.join("reproduce.log"));
                let output_extensions = &[".png", ".jpg", ".svg", ".csv", ".json", ".txt"];
                for entry in walkdir::WalkDir::new(submission_path) {
                    let entry = entry.map_err(|e| {
                        crate::error::Paper2CodesError::Validation(format!("WalkDir error: {}", e))
                    })?;
                    if entry.file_type().is_file() {
                        if let Some(ext) = entry.path().extension() {
                            let ext_str = format!(".{}", ext.to_string_lossy());
                            if output_extensions.contains(&ext_str.as_str()) {
                                files.push(entry.path().to_path_buf());
                            }
                        }
                    }
                }
            }
        }

        Ok(files)
    }
}

/// Parse judge response to extract score with improved pattern matching
/// Returns (score, explanation) where score is 0.0 or 1.0
fn parse_judge_response(response: &str) -> Result<(f64, String)> {
    let response_lower = response.to_lowercase();

    // Try multiple patterns to find the score (ordered by specificity)
    let patterns = vec![
        (r"#\s*score\s*:?\s*([01])\s*$", 10), // # Score: 1 (at end of line, highest priority)
        (r"#\s*score\s*:?\s*([01])", 9),      // # Score: 1
        (r"score\s*:?\s*([01])\s*$", 8),      // Score: 1 (at end of line)
        (r"score\s*:?\s*(?:is\s*)?([01])", 7), // Score: 1 or Score is 1
        (r"score\s*=\s*([01])", 6),           // Score = 1
        (r"grade\s*:?\s*([01])", 5),          // Grade: 1
        (r"result\s*:?\s*([01])", 4),         // Result: 1
        (r"final\s+score\s*:?\s*([01])", 8),  // Final score: 1
    ];

    let mut best_match: Option<(f64, usize, usize)> = None; // (score, priority, position)

    for (pattern, priority) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(cap) = re.captures(&response_lower) {
                if let Ok(score) = cap[1].parse::<f64>() {
                    let match_pos = cap.get(0).map(|m| m.start()).unwrap_or(0);

                    // Keep the highest priority match, or if same priority, the first one
                    if best_match.map(|(_, p, _)| priority > p).unwrap_or(true) {
                        best_match = Some((score, priority, match_pos));
                    }
                }
            }
        }
    }

    if let Some((score, _, score_pos)) = best_match {
        // Extract the full explanation section with context around the score
        let start = score_pos.saturating_sub(300);
        let end = (score_pos + 300).min(response.len());
        let explanation = if start < end {
            response[start..end].to_string()
        } else {
            response.to_string()
        };
        return Ok((score, explanation));
    }

    // Fallback: Look for explicit "Score: 0" or "Score: 1" patterns (case insensitive)
    if response_lower.contains("score: 1")
        || response_lower.contains("score is 1")
        || response_lower.contains("final score: 1")
    {
        return Ok((1.0, response.to_string()));
    }
    if response_lower.contains("score: 0")
        || response_lower.contains("score is 0")
        || response_lower.contains("final score: 0")
    {
        return Ok((0.0, response.to_string()));
    }

    // Default to 0 if can't parse, but include full response for debugging
    // This helps identify cases where the LLM response format needs adjustment
    Ok((
        0.0,
        format!(
            "Could not parse score from response. Full response: {}",
            response
        ),
    ))
}
