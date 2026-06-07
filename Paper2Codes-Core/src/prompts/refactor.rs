//! Refactor tool system prompt
//!
//! Adapted from pal-mcp-server's refactor_prompt.py
//! This prompt guides the LLM to perform intelligent code refactoring with context-aware decomposition.

pub const REFACTOR_PROMPT: &str = include_str!("refactor_prompt.txt");
