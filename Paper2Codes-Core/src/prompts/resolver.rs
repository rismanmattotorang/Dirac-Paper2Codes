//! Prompt resolver utilities
//!
//! Provides helper functions for retrieving prompts using the tool catalog,
//! while gracefully falling back to the original constants when metadata is
//! unavailable. This keeps agent code concise and ensures a single source of
//! truth for tool configuration.

use super::PromptType;

/// Resolve the system prompt for a given tool using the catalog metadata.
///
/// # Arguments
/// * `tool_name` - Canonical tool identifier in the catalog.
/// * `prompt_type` - Fallback prompt type when the catalog does not contain
///   metadata for the tool.
pub fn resolve_tool_prompt(tool_name: &str, prompt_type: PromptType) -> &'static str {
    crate::agents::get_tool_spec(tool_name)
        .and_then(|spec| spec.system_prompt())
        .unwrap_or_else(|| prompt_type.get_prompt())
}

/// Convenience helpers for common prompts used across agents.
pub fn resolve_analyze_prompt() -> &'static str {
    resolve_tool_prompt("analyze", PromptType::Analyze)
}

pub fn resolve_planner_prompt() -> &'static str {
    resolve_tool_prompt("planner", PromptType::Planner)
}

/// Resolve the structured code generation protocol prompt.
pub fn resolve_generate_code_prompt() -> &'static str {
    PromptType::GenerateCode.get_prompt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::{ANALYZE_PROMPT, PLANNER_PROMPT};

    #[test]
    fn resolve_known_tool_uses_catalog() {
        let prompt = resolve_analyze_prompt();
        assert_eq!(prompt, ANALYZE_PROMPT);
    }

    #[test]
    fn resolve_unknown_tool_falls_back() {
        let prompt = resolve_tool_prompt("non-existent", PromptType::Planner);
        assert_eq!(prompt, PLANNER_PROMPT);
    }
}
