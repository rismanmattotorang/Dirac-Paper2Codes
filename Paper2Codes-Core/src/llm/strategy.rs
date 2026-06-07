//! Advanced multi-LLM strategy system
//!
//! Provides sophisticated LLM routing with task-specific preferences, adaptive strategies,
//! and intelligent result merging.

use crate::domain::ComputationalDomain;
use crate::error::Result;
use crate::llm::{LLMClient, LLMRequest, LLMResponse};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use tracing::{debug, warn};

/// Task types for LLM routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    CodeDetection,
    CodeImprovement,
    CodeGeneration,
    DomainSpecificGeneration(ComputationalDomain),
    Documentation,
    BugFixing,
    PerformanceOptimization,
    SafetyEnhancement,
    TestGeneration,
    DomainDetection,
    Planning,
    Analysis,
    Verification,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CodeDetection => write!(f, "Code Detection"),
            Self::CodeImprovement => write!(f, "Code Improvement"),
            Self::CodeGeneration => write!(f, "Code Generation"),
            Self::DomainSpecificGeneration(domain) => {
                write!(f, "Domain-Specific Generation ({})", domain)
            }
            Self::Documentation => write!(f, "Documentation"),
            Self::BugFixing => write!(f, "Bug Fixing"),
            Self::PerformanceOptimization => write!(f, "Performance Optimization"),
            Self::SafetyEnhancement => write!(f, "Safety Enhancement"),
            Self::TestGeneration => write!(f, "Test Generation"),
            Self::DomainDetection => write!(f, "Domain Detection"),
            Self::Planning => write!(f, "Planning"),
            Self::Analysis => write!(f, "Analysis"),
            Self::Verification => write!(f, "Verification"),
        }
    }
}

/// Strategy for selecting LLM based on task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmStrategy {
    OpenAiOnly,
    ClaudeOnly,
    OpenAiFirstClaudeSecond,
    ClaudeFirstOpenAiSecond,
    CompareAndMerge,
    Adaptive {
        code_detection: AdaptivePreference,
        code_improvement: AdaptivePreference,
        code_generation: AdaptivePreference,
        documentation: AdaptivePreference,
        bug_fixing: AdaptivePreference,
        planning: AdaptivePreference,
        analysis: AdaptivePreference,
        verification: AdaptivePreference,
    },
}

/// Preference for adaptive strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptivePreference {
    pub openai_weight: f64,
    pub claude_weight: f64,
}

impl Default for AdaptivePreference {
    fn default() -> Self {
        Self {
            openai_weight: 0.5,
            claude_weight: 0.5,
        }
    }
}

impl AdaptivePreference {
    pub fn prefer_openai() -> Self {
        Self {
            openai_weight: 0.7,
            claude_weight: 0.3,
        }
    }

    pub fn prefer_claude() -> Self {
        Self {
            openai_weight: 0.3,
            claude_weight: 0.7,
        }
    }
}

impl Default for LlmStrategy {
    fn default() -> Self {
        Self::CompareAndMerge
    }
}

impl LlmStrategy {
    /// Get client preference for a task
    pub fn get_preference(&self, task: &TaskType) -> LlmClientPreference {
        match self {
            LlmStrategy::OpenAiOnly => LlmClientPreference::UseOpenAi,
            LlmStrategy::ClaudeOnly => LlmClientPreference::UseClaude,
            LlmStrategy::OpenAiFirstClaudeSecond => LlmClientPreference::PreferOpenAi,
            LlmStrategy::ClaudeFirstOpenAiSecond => LlmClientPreference::PreferClaude,
            LlmStrategy::CompareAndMerge => LlmClientPreference::Both,
            LlmStrategy::Adaptive { .. } => {
                // Default adaptive logic - can be enhanced
                match task {
                    TaskType::Analysis | TaskType::Documentation => {
                        LlmClientPreference::PreferClaude
                    }
                    TaskType::CodeGeneration | TaskType::BugFixing => {
                        LlmClientPreference::PreferOpenAi
                    }
                    _ => LlmClientPreference::Both,
                }
            }
        }
    }
}

/// Enum indicating which LLM client(s) to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmClientPreference {
    UseOpenAi,
    UseClaude,
    PreferOpenAi,
    PreferClaude,
    Both,
}

/// Enhanced multi-LLM client with strategy support
pub struct StrategyLLMClient {
    openai: Option<Arc<dyn LLMClient>>,
    claude: Option<Arc<dyn LLMClient>>,
    strategy: LlmStrategy,
}

impl StrategyLLMClient {
    pub fn new(
        openai: Option<Arc<dyn LLMClient>>,
        claude: Option<Arc<dyn LLMClient>>,
        strategy: LlmStrategy,
    ) -> Self {
        Self {
            openai,
            claude,
            strategy,
        }
    }

    /// Generate with strategy
    pub async fn generate(&self, request: LLMRequest, task_type: TaskType) -> Result<LLMResponse> {
        let preference = self.strategy.get_preference(&task_type);

        match preference {
            LlmClientPreference::UseOpenAi => {
                self.openai
                    .as_ref()
                    .ok_or_else(|| {
                        crate::error::Paper2CodesError::Validation(
                            "OpenAI client not available".to_string(),
                        )
                    })?
                    .complete(request)
                    .await
            }
            LlmClientPreference::UseClaude => {
                self.claude
                    .as_ref()
                    .ok_or_else(|| {
                        crate::error::Paper2CodesError::Validation(
                            "Claude client not available".to_string(),
                        )
                    })?
                    .complete(request)
                    .await
            }
            LlmClientPreference::PreferOpenAi => {
                if let Some(client) = &self.openai {
                    match client.complete(request.clone()).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            warn!("OpenAI failed, falling back to Claude: {}", e);
                            self.claude
                                .as_ref()
                                .ok_or_else(|| {
                                    crate::error::Paper2CodesError::Validation(
                                        "Claude client not available for fallback".to_string(),
                                    )
                                })?
                                .complete(request)
                                .await
                        }
                    }
                } else if let Some(client) = &self.claude {
                    client.complete(request).await
                } else {
                    Err(crate::error::Paper2CodesError::Validation(
                        "No LLM clients available".to_string(),
                    ))
                }
            }
            LlmClientPreference::PreferClaude => {
                if let Some(client) = &self.claude {
                    match client.complete(request.clone()).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            warn!("Claude failed, falling back to OpenAI: {}", e);
                            self.openai
                                .as_ref()
                                .ok_or_else(|| {
                                    crate::error::Paper2CodesError::Validation(
                                        "OpenAI client not available for fallback".to_string(),
                                    )
                                })?
                                .complete(request)
                                .await
                        }
                    }
                } else if let Some(client) = &self.openai {
                    client.complete(request).await
                } else {
                    Err(crate::error::Paper2CodesError::Validation(
                        "No LLM clients available".to_string(),
                    ))
                }
            }
            LlmClientPreference::Both => {
                // Compare and merge
                self.generate_with_merge(request, task_type).await
            }
        }
    }

    async fn generate_with_merge(
        &self,
        request: LLMRequest,
        task_type: TaskType,
    ) -> Result<LLMResponse> {
        let openai_fut = if let Some(client) = &self.openai {
            Some(client.complete(request.clone()))
        } else {
            None
        };

        let claude_fut = if let Some(client) = &self.claude {
            Some(client.complete(request.clone()))
        } else {
            None
        };

        let (openai_result, claude_result) = match (openai_fut, claude_fut) {
            (Some(o), Some(c)) => {
                let (o_res, c_res) = tokio::join!(o, c);
                (Some(o_res), Some(c_res))
            }
            (Some(o), None) => {
                let o_res = o.await;
                (Some(o_res), None)
            }
            (None, Some(c)) => {
                let c_res = c.await;
                (None, Some(c_res))
            }
            (None, None) => {
                return Err(crate::error::Paper2CodesError::Validation(
                    "No LLM clients available".to_string(),
                ));
            }
        };

        match (openai_result, claude_result) {
            (Some(Ok(openai)), Some(Ok(claude))) => {
                debug!(
                    "Both LLMs succeeded, merging results for task: {}",
                    task_type
                );
                Ok(merge_responses(openai, claude))
            }
            (Some(Ok(openai)), _) => {
                debug!("Only OpenAI succeeded for task: {}", task_type);
                Ok(openai)
            }
            (_, Some(Ok(claude))) => {
                debug!("Only Claude succeeded for task: {}", task_type);
                Ok(claude)
            }
            (Some(Err(e1)), Some(Err(e2))) => Err(crate::error::Paper2CodesError::Validation(
                format!("Both LLMs failed: OpenAI: {}, Claude: {}", e1, e2),
            )),
            (Some(Err(e)), None) | (None, Some(Err(e))) => Err(e),
            (None, None) => Err(crate::error::Paper2CodesError::Validation(
                "No LLM clients available".to_string(),
            )),
        }
    }
}

/// Merge responses from two LLMs
fn merge_responses(openai: LLMResponse, claude: LLMResponse) -> LLMResponse {
    // Simple merge: prefer longer, more detailed response
    // In production, this could use another LLM call to intelligently merge
    if openai.content.len() >= claude.content.len() {
        openai
    } else {
        claude
    }
}
