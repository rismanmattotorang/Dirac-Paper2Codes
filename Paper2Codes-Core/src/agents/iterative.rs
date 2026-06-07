//! Iterative agent with ReAct loop
//!
//! An agent that continuously works on a task using a ReAct (Reasoning + Acting) loop
//! with tool-based execution until a time limit is reached.

use crate::agents::base::{Agent, AgentContext, AgentResponse, AgentType};
use crate::agents::tools::ToolManager;
use crate::error::Result;
use crate::llm::{LLMRequest, LLMRouter, Message, MessageRole};
use crate::types::Task;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Metrics for iterative agent
#[derive(Debug, Clone, Default)]
pub struct IterativeAgentMetrics {
    pub total_tokens: usize,
    pub api_calls: usize,
    pub tool_calls: usize,
    pub steps: usize,
}

/// Iterative agent that works continuously until time limit
pub struct IterativeAgent {
    llm_router: Arc<LLMRouter>,
    max_steps: usize,
    tool_manager: Arc<Mutex<ToolManager>>,
    metrics: Arc<Mutex<IterativeAgentMetrics>>,
    system_prompt: String,
    continue_message: String,
}

impl IterativeAgent {
    /// Create a new iterative agent
    pub fn new(llm_router: Arc<LLMRouter>, work_dir: PathBuf, max_steps: usize) -> Self {
        let mut tool_manager = ToolManager::new();

        // Register default tools
        tool_manager.register_tool(Box::new(crate::agents::tools::FileReaderTool));
        tool_manager.register_tool(Box::new(crate::agents::tools::BashTool::new(
            work_dir.clone(),
        )));
        tool_manager.register_tool(Box::new(crate::agents::tools::PythonTool::new(work_dir)));

        let system_prompt = "You are an expert software developer tasked with implementing research papers. \
                             You have access to tools to read files, execute commands, and run Python code. \
                             Use these tools to explore, understand, and implement the paper requirements. \
                             Work systematically and continuously until the task is complete. \
                             When you need to use a tool, format it as: tool_name(arguments).";

        let continue_message =
            "Continue working on the task. Use available tools to make progress.";

        Self {
            llm_router,
            max_steps,
            tool_manager: Arc::new(Mutex::new(tool_manager)),
            metrics: Arc::new(Mutex::new(IterativeAgentMetrics::default())),
            system_prompt: system_prompt.to_string(),
            continue_message: continue_message.to_string(),
        }
    }

    /// Run the iterative agent loop
    pub async fn run_iterative_loop(
        &self,
        task: &Task,
        _context: &AgentContext,
        time_limit: Duration,
    ) -> Result<AgentResponse> {
        let start_time = Instant::now();
        let deadline = start_time + time_limit;

        let mut messages = vec![
            Message {
                role: MessageRole::System,
                content: self.system_prompt.clone(),
            },
            Message {
                role: MessageRole::User,
                content: format!(
                    "Task: {}\n\nYou have {} hours to complete this task. Start working on it now.",
                    task.description,
                    time_limit.as_secs() / 3600
                ),
            },
        ];

        let mut step_count = 0;

        while step_count < self.max_steps {
            let time_remaining = deadline.saturating_duration_since(Instant::now());
            if time_remaining.as_secs() == 0 {
                warn!("Time limit exceeded");
                break;
            }

            step_count += 1;
            debug!("Iterative agent step {}/{}", step_count, self.max_steps);

            // Get LLM response using router
            let request = LLMRequest::new(
                messages.clone(),
                self.llm_router.config.agents.coding_model.clone(),
            )
            .with_temperature(0.7)
            .with_max_tokens(2048);

            let response = match tokio::time::timeout(
                time_remaining,
                self.llm_router.complete(request, Some(task)),
            )
            .await
            {
                Ok(Ok(resp)) => resp,
                Ok(Err(e)) => {
                    warn!("LLM call failed: {}", e);
                    continue;
                }
                Err(_) => {
                    warn!("LLM call timed out");
                    break;
                }
            };

            // Update metrics
            {
                let mut metrics = self.metrics.lock().await;
                metrics.api_calls += 1;
                metrics.total_tokens += response.tokens_used.unwrap_or(0) as usize;
                metrics.steps = step_count;
            }

            let assistant_message = response.content;
            messages.push(Message {
                role: MessageRole::Assistant,
                content: assistant_message.clone(),
            });

            // Parse and execute tool calls
            let tool_manager = self.tool_manager.lock().await;
            let tool_calls = tool_manager.parse_tool_calls(&assistant_message);
            drop(tool_manager);

            let mut tool_results = Vec::new();
            for tool_call in tool_calls {
                let tool_manager = self.tool_manager.lock().await;
                let result = tool_manager.execute_tool(&tool_call).await;
                drop(tool_manager);

                {
                    let mut metrics = self.metrics.lock().await;
                    metrics.tool_calls += 1;
                }

                let result_message = match result {
                    Ok(output) => format!(
                        "Tool '{}' executed successfully:\n{}",
                        tool_call.tool, output
                    ),
                    Err(e) => format!("Tool '{}' failed: {}", tool_call.tool, e),
                };
                tool_results.push(result_message);
            }

            // Add tool results or continue message
            if !tool_results.is_empty() {
                messages.push(Message {
                    role: MessageRole::User,
                    content: tool_results.join("\n\n"),
                });
            } else {
                messages.push(Message {
                    role: MessageRole::User,
                    content: self.continue_message.clone(),
                });
            }

            // Manage context window
            if messages.len() > 20 {
                messages = vec![messages[0].clone()]
                    .into_iter()
                    .chain(messages.iter().skip(messages.len() - 19).cloned())
                    .collect();
            }
        }

        let duration = start_time.elapsed();
        let metrics = self.metrics.lock().await;

        Ok(AgentResponse {
            task_id: task.id,
            result: crate::types::AgentResult::Verification(crate::types::VerificationReport {
                module_id: None,
                issues: vec![],
                passed: true,
                metrics: Default::default(),
            }),
            metadata: crate::agents::base::ResponseMetadata {
                duration_ms: duration.as_millis() as u64,
                tokens_used: Some(metrics.total_tokens as u32),
            },
        })
    }
}

#[async_trait::async_trait]
impl Agent for IterativeAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        // For iterative agent, we run the loop with a time limit
        let time_limit = Duration::from_secs(3600); // 1 hour default
        self.run_iterative_loop(task, context, time_limit).await
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Coding
    }
}
