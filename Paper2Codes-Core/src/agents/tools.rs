//! Agent tools for iterative agent execution
use crate::agents::ToolSpec;
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Trait for agent tools
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    async fn execute(&self, args: &str) -> Result<String>;
}

/// File reader tool
pub struct FileReaderTool;

#[async_trait]
impl Tool for FileReaderTool {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn description(&self) -> String {
        "Read and return the contents of a file".to_string()
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let path = PathBuf::from(args.trim());
        tokio::fs::read_to_string(&path).await.map_err(|e| {
            crate::error::Paper2CodesError::Validation(format!(
                "Failed to read file {}: {}",
                path.display(),
                e
            ))
        })
    }
}

/// Bash command executor tool
pub struct BashTool {
    work_dir: PathBuf,
}

impl BashTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> String {
        "bash".to_string()
    }

    fn description(&self) -> String {
        "Execute a bash command in the working directory".to_string()
    }

    async fn execute(&self, args: &str) -> Result<String> {
        use tokio::process::Command;
        let output = Command::new("sh")
            .arg("-c")
            .arg(args)
            .current_dir(&self.work_dir)
            .output()
            .await
            .map_err(|e| {
                crate::error::Paper2CodesError::Validation(format!(
                    "Failed to execute command: {}",
                    e
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(crate::error::Paper2CodesError::Validation(format!(
                "Command failed: {}",
                stderr
            )))
        }
    }
}

/// Python executor tool
pub struct PythonTool {
    work_dir: PathBuf,
}

impl PythonTool {
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }
}

#[async_trait]
impl Tool for PythonTool {
    fn name(&self) -> String {
        "python".to_string()
    }

    fn description(&self) -> String {
        "Execute Python code".to_string()
    }

    async fn execute(&self, args: &str) -> Result<String> {
        // Create temporary Python file
        let temp_file = self
            .work_dir
            .join(format!("temp_{}.py", uuid::Uuid::new_v4()));
        tokio::fs::write(&temp_file, args).await?;

        use tokio::process::Command;
        let output = Command::new("python3")
            .arg(&temp_file)
            .current_dir(&self.work_dir)
            .output()
            .await
            .map_err(|e| {
                crate::error::Paper2CodesError::Validation(format!(
                    "Failed to execute Python: {}",
                    e
                ))
            })?;

        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_file).await;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            Ok(stdout.to_string())
        } else {
            Err(crate::error::Paper2CodesError::Validation(format!(
                "Python execution failed: {}",
                stderr
            )))
        }
    }
}

/// Tool manager
pub struct ToolManager {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn parse_tool_calls(&self, text: &str) -> Vec<ToolCall> {
        // Parse tool calls - look for patterns like `tool_name(args)` or `tool_name("args")`
        // Supports both quoted and unquoted arguments
        let mut calls = Vec::new();

        // Pattern to match tool_name(args) where args can be quoted or unquoted
        // This regex matches: tool_name("quoted args") or tool_name(unquoted_args)
        let tool_pattern = match regex::Regex::new(r"(\w+)\s*\(\s*([^)]*)\s*\)") {
            Ok(re) => re,
            Err(e) => {
                tracing::warn!("Failed to compile tool call regex pattern: {}", e);
                return calls;
            }
        };

        for cap in tool_pattern.captures_iter(text) {
            if let (Some(tool_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                let tool_name = tool_match.as_str().to_string();
                let args_str = args_match.as_str().trim();

                // Check if tool exists before adding
                if self.get_tool(&tool_name).is_some() {
                    // Remove quotes if present
                    let cleaned_args = if (args_str.starts_with('"') && args_str.ends_with('"'))
                        || (args_str.starts_with('\'') && args_str.ends_with('\''))
                    {
                        &args_str[1..args_str.len() - 1]
                    } else {
                        args_str
                    };

                    calls.push(ToolCall {
                        tool: tool_name,
                        args: cleaned_args.to_string(),
                    });
                } else {
                    tracing::debug!("Unknown tool in text: {}", tool_name);
                }
            }
        }

        calls
    }

    pub async fn execute_tool(&self, call: &ToolCall) -> Result<String> {
        if let Some(tool) = self.get_tool(&call.tool) {
            tool.execute(&call.args).await
        } else {
            Err(crate::error::Paper2CodesError::Validation(format!(
                "Unknown tool: {}",
                call.tool
            )))
        }
    }

    /// Expose the catalogued metadata for all known tools.
    pub fn available_specs(&self) -> &'static [ToolSpec] {
        crate::agents::TOOL_SPECS
    }
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: String,
}
