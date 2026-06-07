//! Structured code generation protocol prompt
//!
//! Adapted from pal-mcp-server's generate_code_prompt.py.
//! Enables capability-aware code generation when large changes are requested.

pub const GENERATE_CODE_PROMPT: &str = include_str!("generate_code_prompt.txt");
