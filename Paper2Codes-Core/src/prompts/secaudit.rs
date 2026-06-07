//! SecAudit tool system prompt
//!
//! Adapted from pal-mcp-server's secaudit_prompt.py
//! This prompt guides the LLM to perform comprehensive security audits with OWASP Top 10 coverage.

pub const SECAUDIT_PROMPT: &str = include_str!("secaudit_prompt.txt");
