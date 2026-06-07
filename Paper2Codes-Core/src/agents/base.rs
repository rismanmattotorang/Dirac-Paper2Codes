use crate::error::Result;
use crate::llm::LLMRouter;
use crate::retrieval::CPREngine;
use crate::types::{ImplementationPlan, Paper, Repository, Task};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Agent: Send + Sync {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse>;
    fn agent_type(&self) -> AgentType;
}

#[derive(Debug, Clone)]
pub enum AgentType {
    Planning,
    Analysis,
    Coding,
    Verification,
}

pub struct AgentContext {
    pub paper: Arc<Paper>,
    pub plan: Option<Arc<ImplementationPlan>>,
    pub repository: Arc<Repository>,
    pub cpr_engine: Arc<dyn CPREngine>,
    pub llm_router: Arc<LLMRouter>,
}

pub struct AgentResponse {
    pub task_id: uuid::Uuid,
    pub result: crate::types::AgentResult,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub duration_ms: u64,
    pub tokens_used: Option<u32>,
}
