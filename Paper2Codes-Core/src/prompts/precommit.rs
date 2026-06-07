//! Precommit review tool system prompt
//!
//! Adapted from pal-mcp-server's precommit_prompt.py.
//! Guides rigorous pre-commit, PR-style analysis before landing changes.

pub const PRECOMMIT_PROMPT: &str = include_str!("precommit_prompt.txt");
