use crate::error::{ExecutionError, Result};
use crate::types::CodeModule;
use regex::Regex;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Hardening policy applied to Docker-based execution of untrusted, generated
/// code (Phase 2). Defaults are locked down: no capabilities, read-only root
/// filesystem, no privilege escalation, a non-root user, a bounded process
/// count, and a small writable tmpfs. Network isolation and a read-only code
/// mount are always applied by the runner in addition to these flags.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub read_only_rootfs: bool,
    pub drop_all_capabilities: bool,
    pub no_new_privileges: bool,
    pub pids_limit: Option<u32>,
    /// `uid:gid` to run as (e.g. `"65534:65534"` = nobody).
    pub run_as_user: Option<String>,
    /// Size of a writable `/tmp` tmpfs in MB (so a read-only rootfs can still
    /// write scratch files).
    pub tmpfs_tmp_mb: Option<u32>,
    /// Disable swap (set `--memory-swap` equal to the memory limit).
    pub disable_swap: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            read_only_rootfs: true,
            drop_all_capabilities: true,
            no_new_privileges: true,
            pids_limit: Some(256),
            run_as_user: Some("65534:65534".to_string()),
            tmpfs_tmp_mb: Some(64),
            disable_swap: true,
        }
    }
}

/// Build the `docker run` security flags for a policy. Pure (no I/O) for testing.
pub fn docker_security_flags(policy: &SandboxPolicy) -> Vec<String> {
    let mut flags = Vec::new();
    if policy.read_only_rootfs {
        flags.push("--read-only".to_string());
    }
    if policy.drop_all_capabilities {
        flags.push("--cap-drop".to_string());
        flags.push("ALL".to_string());
    }
    if policy.no_new_privileges {
        flags.push("--security-opt".to_string());
        flags.push("no-new-privileges".to_string());
    }
    if let Some(pids) = policy.pids_limit {
        flags.push("--pids-limit".to_string());
        flags.push(pids.to_string());
    }
    if let Some(user) = &policy.run_as_user {
        flags.push("--user".to_string());
        flags.push(user.clone());
    }
    if let Some(mb) = policy.tmpfs_tmp_mb {
        flags.push("--tmpfs".to_string());
        flags.push(format!("/tmp:rw,size={}m", mb));
    }
    flags
}

#[derive(Clone)]
pub struct SandboxRunner {
    timeout: Duration,
    work_dir: PathBuf,
    use_docker: bool,
    docker_image: Option<String>,
    memory_limit_mb: Option<usize>,
    cpu_limit: Option<f32>,
    policy: SandboxPolicy,
}

impl SandboxRunner {
    pub fn new(work_dir: PathBuf) -> Self {
        Self {
            timeout: Duration::from_secs(300),
            work_dir,
            use_docker: false,
            docker_image: None,
            memory_limit_mb: None,
            cpu_limit: None,
            policy: SandboxPolicy::default(),
        }
    }

    /// Override the (hardened by default) sandbox policy.
    pub fn with_sandbox_policy(mut self, policy: SandboxPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable Docker-based execution (optional)
    pub fn with_docker(mut self, image: &str) -> Self {
        self.use_docker = true;
        self.docker_image = Some(image.to_string());
        self
    }

    /// Set memory limit in megabytes
    pub fn with_memory_limit(mut self, limit_mb: usize) -> Self {
        self.memory_limit_mb = Some(limit_mb);
        self
    }

    /// Set CPU limit (e.g., 1.0 = 1 CPU, 0.5 = half CPU)
    pub fn with_cpu_limit(mut self, limit: f32) -> Self {
        self.cpu_limit = Some(limit);
        self
    }

    pub async fn execute_code(
        &self,
        code: &CodeModule,
        input: Option<&str>,
    ) -> Result<crate::execution::runner::ExecutionResult> {
        // Use Docker if enabled, otherwise use local execution
        if self.use_docker {
            return self.execute_in_docker(code, input).await;
        }

        // Create temporary file for code
        let file_path = self.work_dir.join(&code.file_path);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to create directory: {}", e))
            })?;
        }

        // Write code to file
        tokio::fs::write(&file_path, &code.content)
            .await
            .map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to write code file: {}", e))
            })?;

        // Execute based on language
        let start = std::time::Instant::now();
        let result = match code.language {
            crate::types::ProgrammingLanguage::Python => {
                self.execute_python(&file_path, input).await
            }
            crate::types::ProgrammingLanguage::Rust => self.execute_rust(&file_path, input).await,
            crate::types::ProgrammingLanguage::Other(ref lang) => {
                Err(ExecutionError::SandboxFailed(format!("Unsupported language: {}", lang)).into())
            }
        }?;

        let duration = start.elapsed();

        // Cleanup temporary file if it's in a temp directory
        // Note: We keep files in work_dir for debugging, but could clean up here if needed

        Ok(crate::execution::runner::ExecutionResult {
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            duration,
        })
    }

    async fn execute_python(
        &self,
        file_path: &PathBuf,
        input: Option<&str>,
    ) -> Result<ProcessResult> {
        let mut cmd = Command::new("python3");
        cmd.arg(file_path);
        cmd.current_dir(&self.work_dir);

        if let Some(_input_data) = input {
            cmd.stdin(std::process::Stdio::piped());
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to spawn Python process: {}", e))
            })?;

        // Write input if provided
        if let Some(input_data) = input {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(input_data.as_bytes()).await?;
            }
        }

        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| ExecutionError::Timeout)?;

        let output = output.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to execute Python: {}", e))
        })?;

        Ok(ProcessResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    async fn execute_rust(
        &self,
        file_path: &PathBuf,
        input: Option<&str>,
    ) -> Result<ProcessResult> {
        // For Rust, we need to compile first
        let binary_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        let binary_path = self.work_dir.join(binary_name);

        // Compile
        let compile_output = Command::new("rustc")
            .arg(file_path)
            .arg("-o")
            .arg(&binary_path)
            .current_dir(&self.work_dir)
            .output()
            .await
            .map_err(|e| {
                ExecutionError::CompilationFailed(format!("Failed to compile Rust: {}", e))
            })?;

        if !compile_output.status.success() {
            return Err(ExecutionError::CompilationFailed(
                String::from_utf8_lossy(&compile_output.stderr).to_string(),
            )
            .into());
        }

        // Execute
        let mut cmd = Command::new(&binary_path);
        cmd.current_dir(&self.work_dir);

        if let Some(_input_data) = input {
            cmd.stdin(std::process::Stdio::piped());
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to spawn Rust binary: {}", e))
            })?;

        // Write input if provided
        if let Some(input_data) = input {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(input_data.as_bytes()).await?;
            }
        }

        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| ExecutionError::Timeout)?;

        let output = output.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to execute Rust binary: {}", e))
        })?;

        // Cleanup binary
        let _ = tokio::fs::remove_file(&binary_path).await;

        Ok(ProcessResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    pub async fn run_tests(
        &self,
        code: &CodeModule,
    ) -> Result<crate::execution::runner::TestResults> {
        match code.language {
            crate::types::ProgrammingLanguage::Python => self.run_python_tests(code).await,
            crate::types::ProgrammingLanguage::Rust => self.run_rust_tests(code).await,
            crate::types::ProgrammingLanguage::Other(ref lang) => Err(
                ExecutionError::SandboxFailed(format!("Unsupported language for tests: {}", lang))
                    .into(),
            ),
        }
    }

    async fn run_python_tests(
        &self,
        code: &CodeModule,
    ) -> Result<crate::execution::runner::TestResults> {
        // Try to use pytest first, then unittest, then just execute
        let file_path = self.work_dir.join(&code.file_path);

        // Check if pytest is available
        let pytest_check = Command::new("pytest").arg("--version").output().await;

        if let Ok(output) = pytest_check {
            if output.status.success() {
                // Use pytest
                let test_output = timeout(
                    self.timeout,
                    Command::new("pytest")
                        .arg(&file_path)
                        .arg("-v")
                        .current_dir(&self.work_dir)
                        .output(),
                )
                .await
                .map_err(|_| ExecutionError::Timeout)?;

                let test_output = test_output.map_err(|e| {
                    ExecutionError::SandboxFailed(format!("Failed to run pytest: {}", e))
                })?;

                let stdout = String::from_utf8_lossy(&test_output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&test_output.stderr).to_string();

                // Parse pytest output
                let passed = test_output.status.success();
                let total = stdout.matches("PASSED").count() + stdout.matches("FAILED").count();
                let failed = stdout.matches("FAILED").count();

                return Ok(crate::execution::runner::TestResults {
                    passed,
                    total: total.max(1),
                    failed,
                    output: stdout,
                    errors: stderr,
                });
            }
        }

        // Fallback: try unittest
        let unittest_output = timeout(
            self.timeout,
            Command::new("python3")
                .arg("-m")
                .arg("unittest")
                .arg(&file_path)
                .current_dir(&self.work_dir)
                .output(),
        )
        .await;

        if let Ok(Ok(output)) = unittest_output {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let passed = output.status.success();
            let total = stdout.matches("test_").count().max(1);
            let failed = if passed { 0 } else { total };

            return Ok(crate::execution::runner::TestResults {
                passed,
                total,
                failed,
                output: stdout,
                errors: stderr,
            });
        }

        // Final fallback: just execute and check exit code
        let result = self.execute_code(code, None).await?;

        Ok(crate::execution::runner::TestResults {
            passed: result.exit_code == 0,
            total: 1,
            failed: if result.exit_code == 0 { 0 } else { 1 },
            output: result.stdout,
            errors: result.stderr,
        })
    }

    async fn run_rust_tests(
        &self,
        code: &CodeModule,
    ) -> Result<crate::execution::runner::TestResults> {
        // For Rust, use cargo test
        let _file_path = self.work_dir.join(&code.file_path);

        // Create a simple Cargo.toml if needed
        let cargo_toml = self.work_dir.join("Cargo.toml");
        if !cargo_toml.exists() {
            let cargo_content = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#;
            tokio::fs::write(&cargo_toml, cargo_content).await?;
        }

        let output = timeout(
            self.timeout,
            Command::new("cargo")
                .arg("test")
                .current_dir(&self.work_dir)
                .output(),
        )
        .await
        .map_err(|_| ExecutionError::Timeout)?;

        let output = output.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to run Rust tests: {}", e))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse cargo test output more accurately
        let passed = output.status.success();

        // Try to extract test counts from cargo output
        let mut total = 0;
        let mut failed = 0;

        // Look for patterns like "test result: ok. X passed; Y failed"
        let re = Regex::new(r"test result:.*?(\d+)\s+passed.*?(\d+)\s+failed").unwrap();
        if let Some(caps) = re.captures(&stdout) {
            if let (Ok(p), Ok(f)) = (caps[1].parse::<usize>(), caps[2].parse::<usize>()) {
                total = p + f;
                failed = f;
            }
        }

        // Fallback: count test names
        if total == 0 {
            total = stdout.matches("test ").count().max(1);
            failed = if passed { 0 } else { total };
        }

        Ok(crate::execution::runner::TestResults {
            passed,
            total,
            failed,
            output: stdout,
            errors: stderr,
        })
    }

    /// Execute code in Docker container (optional feature)
    pub async fn execute_in_docker(
        &self,
        code: &CodeModule,
        input: Option<&str>,
    ) -> Result<crate::execution::runner::ExecutionResult> {
        if !self.use_docker {
            return Err(ExecutionError::SandboxFailed(
                "Docker execution not enabled. Use with_docker() to enable.".to_string(),
            )
            .into());
        }

        let image = self.docker_image.as_ref().ok_or_else(|| {
            ExecutionError::SandboxFailed("Docker image not specified".to_string())
        })?;

        // Write code to temporary file
        let file_path = self.work_dir.join(&code.file_path);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&file_path, &code.content).await?;

        let start = std::time::Instant::now();

        // Build docker run command with proper path handling
        let work_dir_abs = self.work_dir.canonicalize().map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to canonicalize work directory: {}", e))
        })?;

        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("-v")
            .arg(format!("{}:/workspace:ro", work_dir_abs.display())); // Read-only mount for security

        // Add resource limits if specified
        if let Some(memory_mb) = self.memory_limit_mb {
            cmd.arg("-m").arg(format!("{}m", memory_mb));
            // Disable swap by pinning memory-swap to the memory limit.
            if self.policy.disable_swap {
                cmd.arg("--memory-swap").arg(format!("{}m", memory_mb));
            }
        }
        if let Some(cpu_limit) = self.cpu_limit {
            cmd.arg("--cpus").arg(format!("{}", cpu_limit));
        }

        // Add network isolation (untrusted generated code gets no network).
        cmd.arg("--network").arg("none");

        // Apply the hardening policy (read-only rootfs, dropped capabilities,
        // no-new-privileges, pids limit, non-root user, tmpfs scratch).
        for flag in docker_security_flags(&self.policy) {
            cmd.arg(flag);
        }

        // Set working directory and image
        cmd.arg("-w").arg("/workspace");
        cmd.arg(image);

        // Add execution command based on language
        // Note: file_path is relative to work_dir, so in container it's relative to /workspace
        match code.language {
            crate::types::ProgrammingLanguage::Python => {
                cmd.arg("python3")
                    .arg(format!("/workspace/{}", code.file_path.display()));
            }
            crate::types::ProgrammingLanguage::Rust => {
                // For Rust in Docker, we'd need a Rust image and compile there
                // This is a simplified version - in production, you'd want a proper Rust Docker setup
                return Err(ExecutionError::SandboxFailed(
                    "Rust compilation in Docker requires a Rust-enabled image. Use local execution or configure a Rust Docker image.".to_string()
                ).into());
            }
            _ => {
                return Err(ExecutionError::SandboxFailed(format!(
                    "Language {:?} not supported in Docker execution",
                    code.language
                ))
                .into());
            }
        }

        // Handle input
        if let Some(_input_data) = input {
            cmd.stdin(std::process::Stdio::piped());
        }

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to spawn Docker container: {}", e))
            })?;

        // Write input if provided
        if let Some(input_data) = input {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(input_data.as_bytes()).await?;
            }
        }

        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| ExecutionError::Timeout)?;

        let output = output.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Docker execution failed: {}", e))
        })?;

        let duration = start.elapsed();

        Ok(crate::execution::runner::ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }

    /// Check if Docker is available
    pub async fn is_docker_available() -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get sandbox configuration info
    pub fn get_config_info(&self) -> SandboxConfig {
        SandboxConfig {
            use_docker: self.use_docker,
            docker_image: self.docker_image.clone(),
            timeout_secs: self.timeout.as_secs(),
            memory_limit_mb: self.memory_limit_mb,
            cpu_limit: self.cpu_limit,
        }
    }
}

/// Sandbox configuration information
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub use_docker: bool,
    pub docker_image: Option<String>,
    pub timeout_secs: u64,
    pub memory_limit_mb: Option<usize>,
    pub cpu_limit: Option<f32>,
}

struct ProcessResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn default_policy_is_hardened() {
        let flags = docker_security_flags(&SandboxPolicy::default());
        let joined = flags.join(" ");
        assert!(joined.contains("--read-only"));
        assert!(joined.contains("--cap-drop ALL"));
        assert!(joined.contains("--security-opt no-new-privileges"));
        assert!(joined.contains("--pids-limit 256"));
        assert!(joined.contains("--user 65534:65534"));
        assert!(joined.contains("--tmpfs /tmp:rw,size=64m"));
    }

    #[test]
    fn relaxed_policy_emits_no_flags() {
        let policy = SandboxPolicy {
            read_only_rootfs: false,
            drop_all_capabilities: false,
            no_new_privileges: false,
            pids_limit: None,
            run_as_user: None,
            tmpfs_tmp_mb: None,
            disable_swap: false,
        };
        assert!(docker_security_flags(&policy).is_empty());
    }
}
