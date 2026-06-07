use crate::agents::base::{Agent, AgentContext, AgentResponse, AgentType};
use crate::error::Result;
use crate::llm::{LLMRequest, Message, MessageRole};
use crate::types::{AgentResult, RetrievedContext, Specification, Task};
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Clone)]
pub struct AnalysisAgent;

impl AnalysisAgent {
    pub fn new() -> Self {
        Self
    }

    async fn analyze_module_llm(
        &self,
        context: &AgentContext,
        task: &Task,
        retrieved_contexts: &[RetrievedContext],
    ) -> Result<String> {
        let context_text = retrieved_contexts
            .iter()
            .map(|rc| format!("[{}] {}", rc.segment.section, rc.segment.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let module_id = match &task.task_type {
            crate::types::TaskType::Analysis { module_id } => module_id,
            _ => "unknown",
        };

        let prompt = format!(
            r#"You are an analysis agent that extracts detailed specifications from research papers.

Task: {}
Module ID: {}

Relevant Paper Context:
{}

Please analyze the task and extract detailed specifications including:
1. Function signatures (if applicable) - include parameter types and return types
2. Equations and mathematical formulations - extract all relevant equations from the paper
3. Constraints and requirements - list all constraints, assumptions, and requirements
4. Detailed description - provide a comprehensive description of what needs to be implemented

IMPORTANT: Return ONLY valid JSON. Do not include any markdown formatting, explanations, or additional text outside the JSON object.

Return the specification as a JSON object with this exact structure:
{{
  "description": "detailed description of what needs to be implemented",
  "function_signature": "def function_name(param1: type, param2: type) -> return_type",
  "equations": ["equation1 in LaTeX or mathematical notation", "equation2"],
  "constraints": ["constraint1", "constraint2"]
}}

If a field is not applicable, use null or an empty array. The description field is required.

Generate the specification as JSON only:"#,
            task.description, module_id, context_text
        );

        // Use the enhanced analyze prompt from tool catalog (falls back to constant)
        use crate::prompts::resolve_analyze_prompt;
        let system_prompt = resolve_analyze_prompt();

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: system_prompt.to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            context.llm_router.config.agents.analysis_model.clone(),
        );

        let response = context.llm_router.complete(request, Some(task)).await?;
        Ok(response.content)
    }
}

#[async_trait]
impl Agent for AnalysisAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        let start = std::time::Instant::now();

        // Retrieve relevant context using CPR
        let retrieved_contexts = context
            .cpr_engine
            .retrieve(task, &context.paper, &context.repository, 5)
            .await?;

        // Generate specification using LLM
        let spec_json = self
            .analyze_module_llm(context, task, &retrieved_contexts)
            .await?;

        // Parse specification
        let spec = self.parse_spec_json(&spec_json, task)?;

        let duration = start.elapsed();

        Ok(AgentResponse {
            task_id: task.id,
            result: AgentResult::Analysis(spec),
            metadata: crate::agents::base::ResponseMetadata {
                duration_ms: duration.as_millis() as u64,
                tokens_used: None,
            },
        })
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Analysis
    }
}

impl AnalysisAgent {
    fn parse_spec_json(&self, json_str: &str, task: &Task) -> Result<Specification> {
        use serde_json::Value;

        // Extract JSON from potential markdown code blocks with better handling
        let cleaned_json = self.extract_json_from_markdown(json_str);

        // Try to parse JSON with better error handling
        let parsed: Value = match serde_json::from_str(&cleaned_json) {
            Ok(v) => v,
            Err(e) => {
                // Try to find JSON object in the text if direct parsing fails
                if let Some(json_start) = cleaned_json.find('{') {
                    if let Some(json_end) = cleaned_json.rfind('}') {
                        let json_slice = &cleaned_json[json_start..=json_end];
                        match serde_json::from_str(json_slice) {
                            Ok(v) => v,
                            Err(e2) => {
                                tracing::warn!(
                                    "Failed to parse specification JSON: {} (also tried extracted: {}). Using fallback.",
                                    e, e2
                                );
                                // Return a basic specification with task description
                                return Ok(Specification {
                                    id: Uuid::new_v4().to_string(),
                                    description: task.description.clone(),
                                    function_signature: None,
                                    equations: Vec::new(),
                                    constraints: Vec::new(),
                                });
                            }
                        }
                    } else {
                        tracing::warn!(
                            "Failed to parse specification JSON: {}. Using fallback.",
                            e
                        );
                        return Ok(Specification {
                            id: Uuid::new_v4().to_string(),
                            description: task.description.clone(),
                            function_signature: None,
                            equations: Vec::new(),
                            constraints: Vec::new(),
                        });
                    }
                } else {
                    tracing::warn!("Failed to parse specification JSON: {}. Using fallback.", e);
                    return Ok(Specification {
                        id: Uuid::new_v4().to_string(),
                        description: task.description.clone(),
                        function_signature: None,
                        equations: Vec::new(),
                        constraints: Vec::new(),
                    });
                }
            }
        };

        // Extract fields with validation
        let description = parsed
            .get("description")
            .and_then(|d| d.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&task.description)
            .to_string();

        let function_signature = parsed
            .get("function_signature")
            .and_then(|f| f.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string());

        let equations = parsed
            .get("equations")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let constraints = parsed
            .get("constraints")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Ok(Specification {
            id: Uuid::new_v4().to_string(),
            description,
            function_signature,
            equations,
            constraints,
        })
    }

    /// Extract JSON from markdown code blocks or plain text
    fn extract_json_from_markdown(&self, text: &str) -> String {
        let trimmed = text.trim();

        // Check for markdown code blocks
        if trimmed.starts_with("```") {
            // Find the first newline after ```
            if let Some(first_newline) = trimmed.find('\n') {
                let after_lang = &trimmed[first_newline + 1..];
                // Find the closing ```
                if let Some(end_marker) = after_lang.rfind("```") {
                    return after_lang[..end_marker].trim().to_string();
                }
            }
        }

        // Try to find JSON object boundaries
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                if end > start {
                    return trimmed[start..=end].to_string();
                }
            }
        }

        trimmed.to_string()
    }
}

impl Default for AnalysisAgent {
    fn default() -> Self {
        Self::new()
    }
}
