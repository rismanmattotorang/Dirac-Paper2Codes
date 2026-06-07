use crate::agents::base::{Agent, AgentContext, AgentResponse, AgentType};
use crate::config::Config;
use crate::error::Result;
use crate::types::{AgentResult, Task};
use crate::verification::SACVPipeline;
use async_trait::async_trait;

#[derive(Clone)]
pub struct VerificationAgent {
    pipeline: SACVPipeline,
}

impl VerificationAgent {
    pub fn new() -> Self {
        Self {
            pipeline: SACVPipeline::new(),
        }
    }

    pub fn with_config(config: &Config) -> Self {
        Self {
            pipeline: SACVPipeline::with_config(config),
        }
    }
}

#[async_trait]
impl Agent for VerificationAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        let start = std::time::Instant::now();

        // Extract specifications from plan or task context
        // The plan's specifications() method returns an empty vec, so we prioritize task context
        let specifications = if !task.context.specifications.is_empty() {
            task.context.specifications.clone()
        } else if let Some(plan) = &context.plan {
            // Fallback to plan specifications (though it currently returns empty)
            plan.specifications()
        } else {
            Vec::new()
        };

        if specifications.is_empty() {
            tracing::warn!("No specifications found for verification task {}", task.id);
        }

        // Run verification pipeline
        let report = self
            .pipeline
            .verify(&context.repository, &specifications)
            .await?;

        let duration = start.elapsed();

        Ok(AgentResponse {
            task_id: task.id,
            result: AgentResult::Verification(report),
            metadata: crate::agents::base::ResponseMetadata {
                duration_ms: duration.as_millis() as u64,
                tokens_used: None,
            },
        })
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Verification
    }
}

impl Default for VerificationAgent {
    fn default() -> Self {
        Self::new()
    }
}
