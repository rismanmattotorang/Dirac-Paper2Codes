use crate::agents::base::{Agent, AgentContext, AgentResponse, AgentType};
use crate::domain::templates::DomainTemplates;
use crate::domain::ComputationalDomain;
use crate::error::Result;
use crate::llm::{LLMRequest, Message, MessageRole};
use crate::prompts::resolve_generate_code_prompt;
use crate::types::{AgentResult, CodeModule, ModuleStatus, ProgrammingLanguage, RetrievedContext, Task};
use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone)]
pub struct CodingAgent;

const CODING_BASE_PROMPT: &str =
    "You are an expert software engineer who writes clean, correct, and well-documented code.";

impl CodingAgent {
    pub fn new() -> Self {
        Self
    }

    async fn generate_code_llm(
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

        let specifications_text = task
            .context
            .specifications
            .iter()
            .map(|s| format!("- {}", s.description))
            .collect::<Vec<_>>()
            .join("\n");

        // Detect domain from paper content for domain-aware code generation
        let domain_guidance = self.get_domain_guidance(context).await;

        // Determine target language
        let target_language = self.determine_language(task, &domain_guidance);

        // Get domain-specific template if available
        let template_hint = if let Some(domain) = self.detect_domain_from_context(context).await {
            if let Some(template) = DomainTemplates::get_template(domain, &target_language) {
                format!(
                    "\n\nDomain-specific code template reference:\n```{}\n{}\n```",
                    target_language, template
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let module_id = match &task.task_type {
            crate::types::TaskType::Coding { module_id } => module_id,
            _ => "unknown",
        };

        let code_context = task.context.code_context.as_deref().unwrap_or("None");

        let prompt = format!(
            r#"You are a coding agent that generates production-quality code from specifications.

Task: {}
Module ID: {}

{}

Specifications:
{}

Relevant Paper Context:
{}

Existing Code Context:
{}{}

Please generate complete, working code for this module. The code must:
1. Match the specifications exactly - implement all required functionality
2. Follow best practices for {} - use proper style, naming conventions, and patterns
3. Include comprehensive error handling - handle edge cases and invalid inputs
4. Include detailed docstrings/comments - explain the logic, parameters, and return values
5. Be production-ready - no placeholders, no TODO comments, fully functional
6. Follow domain-specific conventions - use appropriate libraries/frameworks from the domain guidance
7. Include type hints/annotations where applicable
8. Be well-structured and maintainable

IMPORTANT: Return ONLY the code. Do not include explanations, markdown formatting, or any text outside the code block. If you need to wrap the code in markdown, use a code block with the appropriate language identifier.

Generate the code:"#,
            task.description,
            module_id,
            domain_guidance,
            specifications_text,
            context_text,
            code_context,
            template_hint,
            target_language
        );

        let system_prompt = format!(
            "{}\n\n{}",
            resolve_generate_code_prompt(),
            CODING_BASE_PROMPT
        );

        let request = LLMRequest::new(
            vec![
                Message {
                    role: MessageRole::System,
                    content: system_prompt,
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            context.llm_router.config.agents.coding_model.clone(),
        );

        let response = context.llm_router.complete(request, Some(task)).await?;
        Ok(response.content)
    }

    /// Extract code from LLM response, removing markdown code blocks if present
    fn extract_code_from_response(&self, response: &str) -> String {
        let trimmed = response.trim();

        if trimmed.is_empty() {
            return String::new();
        }

        // Check if response is wrapped in markdown code blocks
        if trimmed.starts_with("```") {
            // Find the first line break after ```
            if let Some(first_newline) = trimmed.find('\n') {
                let after_lang = &trimmed[first_newline + 1..];
                // Find the closing ```
                if let Some(end_marker) = after_lang.rfind("```") {
                    return after_lang[..end_marker].trim().to_string();
                }
                // If no closing marker, return everything after the first newline
                return after_lang.trim().to_string();
            }
        }

        // Try to find code block boundaries even if not at start
        if let Some(start) = trimmed.find("```") {
            let after_start = &trimmed[start + 3..];
            if let Some(first_newline) = after_start.find('\n') {
                let code_start = &after_start[first_newline + 1..];
                if let Some(end_marker) = code_start.rfind("```") {
                    return code_start[..end_marker].trim().to_string();
                }
            }
        }

        trimmed.to_string()
    }

    /// Get domain guidance for code generation
    async fn get_domain_guidance(&self, context: &AgentContext) -> String {
        if let Some(domain) = self.detect_domain_from_context(context).await {
            DomainTemplates::get_prompt_guidance(domain)
        } else {
            String::new()
        }
    }

    /// Detect domain from paper context using DomainDetector
    async fn detect_domain_from_context(
        &self,
        context: &AgentContext,
    ) -> Option<ComputationalDomain> {
        // Extract text chunks from paper segments
        let text_chunks: Vec<String> = context
            .paper
            .segments
            .iter()
            .take(10) // Use more segments for better detection
            .map(|s| s.content.clone())
            .collect();

        if text_chunks.is_empty() {
            return None;
        }

        // Use DomainDetector for proper domain detection
        // Create a detector without LLM (rule-based only for performance)
        let detector = crate::domain::detector::DomainDetector::default();

        match detector.detect_domain(&text_chunks).await {
            Ok(domain) => {
                if domain != ComputationalDomain::General {
                    Some(domain)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::warn!("Domain detection failed: {}, falling back to None", e);
                None
            }
        }
    }

    /// Determine programming language from task and domain
    fn determine_language(&self, _task: &Task, domain_guidance: &str) -> String {
        // Check if language is specified in task context
        // Otherwise, infer from domain guidance or default to Python
        if domain_guidance.contains("Preferred programming languages:") {
            // Extract first language from guidance
            if let Some(start) = domain_guidance.find("Preferred programming languages:") {
                let rest = &domain_guidance[start..];
                if let Some(end) = rest.find('\n') {
                    let languages = &rest[..end];
                    if let Some(first_lang) = languages.split(',').next() {
                        return first_lang.trim().to_string();
                    }
                }
            }
        }

        // Default to Python
        "python".to_string()
    }
}

#[async_trait]
impl Agent for CodingAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        let start = std::time::Instant::now();

        // Retrieve relevant context using CPR
        let retrieved_contexts = context
            .cpr_engine
            .retrieve(task, &context.paper, &context.repository, 5)
            .await?;

        // Generate code using LLM
        let raw_code = self
            .generate_code_llm(context, task, &retrieved_contexts)
            .await?;

        // Extract actual code from response (remove markdown wrappers if present)
        let code_content = self.extract_code_from_response(&raw_code);

        // Determine language from task, domain, or context
        let domain_guidance = self.get_domain_guidance(context).await;
        let language = self.determine_language(task, &domain_guidance);

        // Determine file path and convert language string to enum
        let (file_path, language_enum) = match &task.task_type {
            crate::types::TaskType::Coding { module_id } => {
                let language_enum = match language.as_str() {
                    "python" | "Python" => ProgrammingLanguage::Python,
                    "rust" | "Rust" => ProgrammingLanguage::Rust,
                    other => ProgrammingLanguage::Other(other.to_string()),
                };
                let extension = match language_enum {
                    ProgrammingLanguage::Python => "py",
                    ProgrammingLanguage::Rust => "rs",
                    ProgrammingLanguage::Other(ref ext) => ext.as_str(),
                };
                (
                    PathBuf::from(format!("src/{}.{}", module_id, extension)),
                    language_enum,
                )
            }
            _ => (PathBuf::from("src/main.py"), ProgrammingLanguage::Python),
        };

        let now = Utc::now();
        let code = CodeModule {
            id: Uuid::new_v4().to_string(),
            repository_id: task.context.repository_id.clone(),
            file_path,
            language: language_enum,
            content: code_content,
            ast: None,
            dependencies: Vec::new(),
            tests: Vec::new(),
            status: ModuleStatus::Completed,
            created_at: now,
            updated_at: now,
        };

        let duration = start.elapsed();

        Ok(AgentResponse {
            task_id: task.id,
            result: AgentResult::Code(code),
            metadata: crate::agents::base::ResponseMetadata {
                duration_ms: duration.as_millis() as u64,
                tokens_used: None,
            },
        })
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Coding
    }
}

impl Default for CodingAgent {
    fn default() -> Self {
        Self::new()
    }
}
