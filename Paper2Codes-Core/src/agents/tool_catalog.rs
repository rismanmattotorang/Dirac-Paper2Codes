//! Tool metadata catalog
//!
//! Provides structured metadata for tools adapted from pal-mcp-server so that
//! Paper2Codes-Core can reason about available capabilities, associated prompts,
//! and default model guidance.

use crate::prompts::PromptType;

/// Categories for model selection inspired by pal-mcp-server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolModelCategory {
    ExtendedReasoning,
    FastResponse,
    Balanced,
}

impl ToolModelCategory {
    /// Human-readable identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            ToolModelCategory::ExtendedReasoning => "extended_reasoning",
            ToolModelCategory::FastResponse => "fast_response",
            ToolModelCategory::Balanced => "balanced",
        }
    }
}

/// Temperature profile semantic borrowed from pal configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureProfile {
    Analytical,
    Balanced,
    Creative,
}

impl TemperatureProfile {
    /// Default numerical value associated with the profile.
    pub fn value(self) -> f32 {
        match self {
            TemperatureProfile::Analytical => 1.0,
            TemperatureProfile::Balanced => 1.0,
            TemperatureProfile::Creative => 1.0,
        }
    }

    /// Label for telemetry/logging.
    pub fn label(self) -> &'static str {
        match self {
            TemperatureProfile::Analytical => "analytical",
            TemperatureProfile::Balanced => "balanced",
            TemperatureProfile::Creative => "creative",
        }
    }
}

/// Normalised specification for a tool.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub primary_prompt: Option<PromptType>,
    pub supplemental_prompts: &'static [PromptType],
    pub temperature: TemperatureProfile,
    pub model_category: ToolModelCategory,
    pub requires_model: bool,
}

impl ToolSpec {
    /// Convenience accessor for the default temperature value.
    pub fn temperature_value(&self) -> f32 {
        self.temperature.value()
    }

    /// Retrieve the primary system prompt string if defined.
    pub fn system_prompt(&self) -> Option<&'static str> {
        self.primary_prompt.map(|prompt| prompt.get_prompt())
    }
}

/// Declarative catalog of supported tools and their metadata.
#[allow(clippy::too_many_arguments)]
pub const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        name: "analyze",
        description: "Performs comprehensive code analysis with systematic investigation and expert validation. Use for architecture, performance, maintainability, and pattern analysis. Guides through structured code review and strategic planning.",
        primary_prompt: Some(PromptType::Analyze),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "chat",
        description: "General chat and collaborative thinking partner for brainstorming, development discussion, getting second opinions, and exploring ideas. Use for ideas, validations, questions, and thoughtful explanations.",
        primary_prompt: Some(PromptType::Chat),
        supplemental_prompts: &[PromptType::GenerateCode],
        temperature: TemperatureProfile::Balanced,
        model_category: ToolModelCategory::FastResponse,
        requires_model: true,
    },
    ToolSpec {
        name: "challenge",
        description: "Prevents reflexive agreement by forcing critical thinking and reasoned analysis when a statement is challenged. Trigger automatically when feedback pushes back on answers or when sanity-checking contentious claims.",
        primary_prompt: None,
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::FastResponse,
        requires_model: false,
    },
    ToolSpec {
        name: "codereview",
        description: "Performs systematic, step-by-step code review with expert validation. Use for comprehensive analysis covering quality, security, performance, and architecture. Guides through structured investigation to ensure thoroughness.",
        primary_prompt: Some(PromptType::CodeReview),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "consensus",
        description: "Builds multi-model consensus through systematic analysis and structured debate. Use for complex decisions, architectural choices, feature proposals, and technology evaluations. Consults multiple perspectives to synthesize recommendations.",
        primary_prompt: Some(PromptType::Consensus),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "debug",
        description: "Performs systematic debugging and root cause analysis for complex issues. Use for mysterious errors, performance regressions, race conditions, memory leaks, and integration problems. Guides hypothesis testing with evidence collection.",
        primary_prompt: Some(PromptType::Debug),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "docgen",
        description: "Generates comprehensive code documentation with systematic analysis of functions, classes, and complexity. Use for documentation generation, API narration, and code understanding while preserving implementation safety checks.",
        primary_prompt: Some(PromptType::DocGen),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: false,
    },
    ToolSpec {
        name: "planner",
        description: "Breaks down complex tasks through interactive, sequential planning with revision and branching capabilities. Use for complex project planning, system design, migration strategies, and architectural decisions.",
        primary_prompt: Some(PromptType::Planner),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Balanced,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: false,
    },
    ToolSpec {
        name: "precommit",
        description: "Validates git changes and repository state before committing with systematic analysis. Use for multi-repository validation, security review, change impact assessment, and completeness verification.",
        primary_prompt: Some(PromptType::Precommit),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "refactor",
        description: "Analyzes code for refactoring opportunities with systematic investigation. Use for code smell detection, decomposition planning, modernization, and maintainability improvements.",
        primary_prompt: Some(PromptType::Refactor),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "secaudit",
        description: "Performs comprehensive security audit with systematic vulnerability assessment. Use for OWASP Top 10 analysis, compliance evaluation, threat modeling, and security architecture review.",
        primary_prompt: Some(PromptType::SecAudit),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "testgen",
        description: "Creates comprehensive test suites with edge case coverage for targeted functions, classes, or modules. Analyzes code paths, identifies failure modes, and generates framework-specific tests.",
        primary_prompt: Some(PromptType::TestGen),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "thinkdeep",
        description: "Performs multi-stage investigation and reasoning for complex problem analysis. Use for architecture decisions, difficult bugs, performance challenges, and security analysis with hypothesis-driven thinking.",
        primary_prompt: Some(PromptType::ThinkDeep),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Creative,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: true,
    },
    ToolSpec {
        name: "tracer",
        description: "Performs systematic code tracing with modes for execution flow or dependency mapping. Use for method execution analysis, call chain tracing, dependency mapping, and architectural understanding.",
        primary_prompt: Some(PromptType::Tracer),
        supplemental_prompts: &[],
        temperature: TemperatureProfile::Analytical,
        model_category: ToolModelCategory::ExtendedReasoning,
        requires_model: false,
    },
];

/// Lookup helper to retrieve a specification by canonical name.
pub fn get_tool_spec(name: &str) -> Option<&'static ToolSpec> {
    TOOL_SPECS.iter().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompts_match_constants() {
        use crate::prompts::{ANALYZE_PROMPT, PLANNER_PROMPT};

        let analyze = get_tool_spec("analyze").expect("analyze spec should exist");
        assert_eq!(
            analyze.system_prompt().unwrap(),
            ANALYZE_PROMPT,
            "Analyze spec should surface analyze prompt"
        );

        let planner = get_tool_spec("planner").expect("planner spec should exist");
        assert_eq!(
            planner.system_prompt().unwrap(),
            PLANNER_PROMPT,
            "Planner spec should surface planner prompt"
        );
    }

    #[test]
    fn catalog_contains_expected_metadata() {
        let docgen = get_tool_spec("docgen").expect("docgen spec should exist");
        assert!(
            docgen.description.contains("documentation"),
            "Docgen description should mention documentation tasks"
        );
        assert!(
            docgen.requires_model == false,
            "Docgen should be model-optional according to catalog"
        );

        let chat = get_tool_spec("chat").expect("chat spec should exist");
        assert_eq!(chat.temperature.label(), "balanced");
        assert_eq!(
            chat.temperature_value(),
            TemperatureProfile::Balanced.value()
        );
        assert!(
            !chat.supplemental_prompts.is_empty(),
            "Chat spec should list supplemental prompts for code generation"
        );
    }
}
