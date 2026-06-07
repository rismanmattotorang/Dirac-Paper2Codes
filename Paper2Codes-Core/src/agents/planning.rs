use crate::agents::base::{Agent, AgentContext, AgentResponse, AgentType};
use crate::error::Result;
use crate::llm::{LLMRequest, Message, MessageRole};
use crate::types::{
    AgentResult, DependencyGraph, ImplementationPlan, Module, ModuleStatus, ModuleType,
    ProgrammingLanguage, Task,
};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PlanningAgent;

impl PlanningAgent {
    pub fn new() -> Self {
        Self
    }

    async fn generate_plan_llm(
        &self,
        context: &AgentContext,
        paper: &crate::types::Paper,
    ) -> Result<String> {
        // Build paper content summary (limit to avoid token limits)
        let paper_content: String = paper
            .segments
            .iter()
            .take(10) // Increased from 5 for better context
            .map(|s| format!("[{}] {}", s.section, s.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Truncate if too long (keep first 8000 chars to leave room for response)
        let paper_content = if paper_content.len() > 8000 {
            format!("{}...", &paper_content[..8000])
        } else {
            paper_content
        };

        let prompt = format!(
            r#"You are a planning agent for code generation from research papers.

Given the following research paper, generate a comprehensive implementation plan in JSON format.

Paper Title: {}
Abstract: {}

Paper Content:
{}

Please generate a structured implementation plan that includes:
1. List of modules to implement (with names, descriptions, types, and dependencies)
2. Dependency graph between modules (specified via dependencies array in each module)
3. Experiments to run (if any are described in the paper)

IMPORTANT: Return ONLY valid JSON. Do not include any markdown formatting, explanations, or additional text outside the JSON object.

Return the plan as a JSON object with the following exact structure:
{{
  "modules": [
    {{
      "name": "module_name",
      "description": "detailed module description",
      "type": "Class|Function|Script|Configuration|Test|Documentation",
      "dependencies": ["dependency_module_name1", "dependency_module_name2"],
      "language": "Python|Rust|Other"
    }}
  ],
  "experiments": [
    {{
      "name": "experiment_name",
      "description": "experiment description"
    }}
  ]
}}

Guidelines:
- Module names should be descriptive and follow naming conventions
- Dependencies should reference other module names in the modules array
- At least one module must be specified
- Module types should be one of: Class, Function, Script, Configuration, Test, Documentation
- Languages should be one of: Python, Rust, or Other (specify the language name)

Generate the implementation plan as JSON only:"#,
            paper.title, paper.abstract_text, paper_content
        );

        // Use the enhanced planner prompt from tool catalog (falls back to constant)
        use crate::prompts::resolve_planner_prompt;
        let system_prompt = resolve_planner_prompt();

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
            context.llm_router.config.agents.planning_model.clone(),
        );

        let response = context.llm_router.complete(request, None).await?;
        Ok(response.content)
    }
}

#[async_trait]
impl Agent for PlanningAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        let start = std::time::Instant::now();

        // Generate plan using LLM
        let plan_json = self.generate_plan_llm(context, &context.paper).await?;

        // Parse JSON response (simplified - in real implementation would use proper JSON parsing)
        let plan = self.parse_plan_json(&plan_json, &context.paper)?;

        let duration = start.elapsed();

        Ok(AgentResponse {
            task_id: task.id,
            result: AgentResult::Plan(plan),
            metadata: crate::agents::base::ResponseMetadata {
                duration_ms: duration.as_millis() as u64,
                tokens_used: None, // Would be extracted from LLM response
            },
        })
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Planning
    }
}

impl PlanningAgent {
    fn parse_plan_json(
        &self,
        json_str: &str,
        paper: &crate::types::Paper,
    ) -> Result<ImplementationPlan> {
        use serde_json::Value;

        // Extract JSON from potential markdown code blocks
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
                                return Err(crate::error::Paper2CodesError::Agent(
                                    crate::error::AgentError::ExecutionFailed(
                                        format!("Failed to parse plan JSON: {} (also tried extracted: {}). Response preview: {}", 
                                        e, e2, &cleaned_json[..cleaned_json.len().min(500)])
                                    )
                                ));
                            }
                        }
                    } else {
                        return Err(crate::error::Paper2CodesError::Agent(
                            crate::error::AgentError::ExecutionFailed(format!(
                                "Failed to parse plan JSON: {}. Response preview: {}",
                                e,
                                &cleaned_json[..cleaned_json.len().min(500)]
                            )),
                        ));
                    }
                } else {
                    return Err(crate::error::Paper2CodesError::Agent(
                        crate::error::AgentError::ExecutionFailed(format!(
                            "Failed to parse plan JSON: {}. Response preview: {}",
                            e,
                            &cleaned_json[..cleaned_json.len().min(500)]
                        )),
                    ));
                }
            }
        };

        let mut modules = Vec::new();

        // Parse modules array
        if let Some(modules_array) = parsed.get("modules").and_then(|m| m.as_array()) {
            for module_obj in modules_array {
                let name = module_obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed_module")
                    .to_string();

                let description = module_obj
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                let module_type = match module_obj.get("type").and_then(|t| t.as_str()) {
                    Some("Class") => ModuleType::Class,
                    Some("Function") => ModuleType::Function,
                    Some("Script") => ModuleType::Script,
                    Some("Configuration") => ModuleType::Configuration,
                    Some("Test") => ModuleType::Test,
                    Some("Documentation") => ModuleType::Documentation,
                    _ => ModuleType::Script,
                };

                let dependencies = module_obj
                    .get("dependencies")
                    .and_then(|d| d.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let language = match module_obj.get("language").and_then(|l| l.as_str()) {
                    Some("Python") => ProgrammingLanguage::Python,
                    Some("Rust") => ProgrammingLanguage::Rust,
                    Some(other) => ProgrammingLanguage::Other(other.to_string()),
                    None => ProgrammingLanguage::Python, // Default
                };

                modules.push(Module {
                    id: Uuid::new_v4().to_string(),
                    name,
                    description,
                    module_type,
                    dependencies,
                    language,
                    status: ModuleStatus::Pending,
                });
            }
        }

        // If no modules were parsed, create at least one default module
        if modules.is_empty() {
            tracing::warn!("No modules parsed from JSON, creating default module");
            modules.push(Module {
                id: Uuid::new_v4().to_string(),
                name: "main".to_string(),
                description: format!("Implementation for: {}", paper.title),
                module_type: ModuleType::Script,
                dependencies: Vec::new(),
                language: ProgrammingLanguage::Python,
                status: ModuleStatus::Pending,
            });
        }

        // Parse experiments
        let mut experiments = Vec::new();
        if let Some(experiments_array) = parsed.get("experiments").and_then(|e| e.as_array()) {
            for exp_obj in experiments_array {
                let name = exp_obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("experiment")
                    .to_string();

                let description = exp_obj
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();

                experiments.push(crate::types::Experiment {
                    id: Uuid::new_v4().to_string(),
                    name,
                    description,
                    expected_results: None,
                });
            }
        }

        // Build dependency graph
        let mut dep_graph = DependencyGraph::new();
        for module in &modules {
            for dep in &module.dependencies {
                // Find dependency module by name
                if let Some(dep_module) = modules.iter().find(|m| m.name == *dep) {
                    dep_graph.add_dependency(module.id.clone(), dep_module.id.clone());
                }
            }
        }

        Ok(ImplementationPlan {
            id: Uuid::new_v4().to_string(),
            modules,
            dependencies: dep_graph,
            experiments,
            metadata: crate::types::PlanMetadata {
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
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

impl Default for PlanningAgent {
    fn default() -> Self {
        Self::new()
    }
}
