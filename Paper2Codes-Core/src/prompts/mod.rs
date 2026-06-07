//! System Prompts Module
//!
//! This module contains system prompts adapted from pal-mcp-server for use in Paper2Codes-Core.
//! These prompts provide structured instructions for LLM agents to perform various code analysis,
//! planning, and generation tasks.

pub mod analyze;
pub mod chat;
pub mod codereview;
pub mod consensus;
pub mod debug;
pub mod docgen;
pub mod generate_code;
pub mod planner;
pub mod precommit;
pub mod refactor;
pub mod resolver;
pub mod secaudit;
pub mod testgen;
pub mod thinkdeep;
pub mod tracer;

pub use analyze::ANALYZE_PROMPT;
pub use chat::CHAT_PROMPT;
pub use codereview::CODEREVIEW_PROMPT;
pub use consensus::CONSENSUS_PROMPT;
pub use debug::DEBUG_PROMPT;
pub use docgen::DOCGEN_PROMPT;
pub use generate_code::GENERATE_CODE_PROMPT;
pub use planner::PLANNER_PROMPT;
pub use precommit::PRECOMMIT_PROMPT;
pub use refactor::REFACTOR_PROMPT;
pub use resolver::{
    resolve_analyze_prompt, resolve_generate_code_prompt, resolve_planner_prompt,
    resolve_tool_prompt,
};
pub use secaudit::SECAUDIT_PROMPT;
pub use testgen::TESTGEN_PROMPT;
pub use thinkdeep::THINKDEEP_PROMPT;
pub use tracer::TRACER_PROMPT;

/// System prompt provider trait for agents
pub trait SystemPromptProvider {
    /// Get the system prompt for this agent/tool
    fn get_system_prompt(&self) -> &'static str;

    /// Get a customized system prompt with additional context
    fn get_customized_prompt(&self, context: &str) -> String {
        format!("{}\n\n{}", self.get_system_prompt(), context)
    }
}

/// Enum for different prompt types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptType {
    Analyze,
    Chat,
    CodeReview,
    Consensus,
    Debug,
    DocGen,
    GenerateCode,
    Planner,
    Precommit,
    Refactor,
    SecAudit,
    TestGen,
    ThinkDeep,
    Tracer,
}

impl PromptType {
    /// Get the system prompt string for this prompt type
    pub fn get_prompt(&self) -> &'static str {
        match self {
            PromptType::Analyze => ANALYZE_PROMPT,
            PromptType::Chat => CHAT_PROMPT,
            PromptType::CodeReview => CODEREVIEW_PROMPT,
            PromptType::Consensus => CONSENSUS_PROMPT,
            PromptType::Debug => DEBUG_PROMPT,
            PromptType::DocGen => DOCGEN_PROMPT,
            PromptType::GenerateCode => GENERATE_CODE_PROMPT,
            PromptType::Planner => PLANNER_PROMPT,
            PromptType::Precommit => PRECOMMIT_PROMPT,
            PromptType::Refactor => REFACTOR_PROMPT,
            PromptType::SecAudit => SECAUDIT_PROMPT,
            PromptType::TestGen => TESTGEN_PROMPT,
            PromptType::ThinkDeep => THINKDEEP_PROMPT,
            PromptType::Tracer => TRACER_PROMPT,
        }
    }

    /// Get the name of the prompt type
    pub fn name(&self) -> &'static str {
        match self {
            PromptType::Analyze => "analyze",
            PromptType::Chat => "chat",
            PromptType::CodeReview => "codereview",
            PromptType::Consensus => "consensus",
            PromptType::Debug => "debug",
            PromptType::DocGen => "docgen",
            PromptType::GenerateCode => "generate_code",
            PromptType::Planner => "planner",
            PromptType::Precommit => "precommit",
            PromptType::Refactor => "refactor",
            PromptType::SecAudit => "secaudit",
            PromptType::TestGen => "testgen",
            PromptType::ThinkDeep => "thinkdeep",
            PromptType::Tracer => "tracer",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_type_get_prompt() {
        for prompt_type in [
            PromptType::Analyze,
            PromptType::Planner,
            PromptType::DocGen,
            PromptType::Chat,
            PromptType::Tracer,
        ] {
            let prompt = prompt_type.get_prompt();
            assert!(
                !prompt.is_empty(),
                "Prompt {:?} should not be empty",
                prompt_type
            );
            assert!(
                prompt.contains("ROLE") || prompt.contains("You are"),
                "Prompt {:?} should contain role guidance",
                prompt_type
            );
        }
    }

    #[test]
    fn test_prompt_type_name() {
        assert_eq!(PromptType::Analyze.name(), "analyze");
        assert_eq!(PromptType::Planner.name(), "planner");
        assert_eq!(PromptType::DocGen.name(), "docgen");
    }
}
