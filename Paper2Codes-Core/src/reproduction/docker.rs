//! Docker-based reproduction system (requires docker feature)
use crate::error::{ExecutionError, Result};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct ReproductionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
    pub output_files: Vec<PathBuf>,
}

pub struct ReproductionSystem {
    docker_image: String,
    timeout: Duration,
    memory_limit_mb: Option<usize>,
    cpu_limit: Option<f32>,
}

impl ReproductionSystem {
    pub fn new() -> Result<Self> {
        Self::with_image("python:3.11")
    }

    pub fn with_image(image: &str) -> Result<Self> {
        // Validate image name format
        if image.trim().is_empty() {
            return Err(ExecutionError::SandboxFailed(
                "Docker image name cannot be empty".to_string(),
            )
            .into());
        }

        // Basic validation: image should contain at least one colon or be a valid name
        if !image.contains(':') && !image.contains('/') && image != "python" && image != "rust" {
            tracing::warn!(
                "Docker image '{}' may not be valid (missing tag or registry)",
                image
            );
        }

        Ok(Self {
            docker_image: image.to_string(),
            timeout: Duration::from_secs(3600), // 1 hour default
            memory_limit_mb: Some(4096),        // 4GB default
            cpu_limit: Some(2.0),               // 2 CPUs default
        })
    }

    /// Validate that the Docker image exists locally or can be pulled
    pub async fn validate_image(&self) -> Result<()> {
        // Check if image exists locally
        let output = Command::new("docker")
            .args(&["images", "-q", &self.docker_image])
            .output()
            .await
            .map_err(|e| {
                ExecutionError::SandboxFailed(format!("Failed to check Docker image: {}", e))
            })?;

        if !output.status.success() {
            return Err(ExecutionError::SandboxFailed(format!(
                "Docker image '{}' not found locally. Please pull it first with: docker pull {}",
                self.docker_image, self.docker_image
            ))
            .into());
        }

        let image_id = String::from_utf8_lossy(&output.stdout);
        if image_id.trim().is_empty() {
            return Err(ExecutionError::SandboxFailed(format!(
                "Docker image '{}' not found locally. Please pull it first with: docker pull {}",
                self.docker_image, self.docker_image
            ))
            .into());
        }

        Ok(())
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        // Validate timeout is reasonable (at least 1 second, at most 24 hours)
        if timeout < Duration::from_secs(1) {
            tracing::warn!(
                "Timeout too short ({}ms), using minimum of 1 second",
                timeout.as_millis()
            );
            self.timeout = Duration::from_secs(1);
        } else if timeout > Duration::from_secs(86400) {
            tracing::warn!(
                "Timeout too long ({}s), capping at 24 hours",
                timeout.as_secs()
            );
            self.timeout = Duration::from_secs(86400);
        } else {
            self.timeout = timeout;
        }
        self
    }

    pub fn with_memory_limit(mut self, limit_mb: usize) -> Self {
        // Validate memory limit is reasonable (at least 128MB, at most 64GB)
        if limit_mb < 128 {
            tracing::warn!(
                "Memory limit too low ({}MB), using minimum of 128MB",
                limit_mb
            );
            self.memory_limit_mb = Some(128);
        } else if limit_mb > 65536 {
            tracing::warn!("Memory limit too high ({}MB), capping at 64GB", limit_mb);
            self.memory_limit_mb = Some(65536);
        } else {
            self.memory_limit_mb = Some(limit_mb);
        }
        self
    }

    pub fn with_cpu_limit(mut self, limit: f32) -> Self {
        // Validate CPU limit is reasonable (at least 0.1, at most 32)
        if limit < 0.1 {
            tracing::warn!("CPU limit too low ({}), using minimum of 0.1", limit);
            self.cpu_limit = Some(0.1);
        } else if limit > 32.0 {
            tracing::warn!("CPU limit too high ({}), capping at 32", limit);
            self.cpu_limit = Some(32.0);
        } else {
            self.cpu_limit = Some(limit);
        }
        self
    }

    /// Check if Docker is available
    pub async fn check_docker_available() -> bool {
        match Command::new("docker").arg("--version").output().await {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub async fn reproduce_submission(
        &self,
        submission_path: &std::path::Path,
    ) -> Result<ReproductionResult> {
        // Validate submission path exists and is a directory
        if !submission_path.exists() {
            return Err(ExecutionError::SandboxFailed(format!(
                "Submission path does not exist: {}",
                submission_path.display()
            ))
            .into());
        }

        if !submission_path.is_dir() {
            return Err(ExecutionError::SandboxFailed(format!(
                "Submission path is not a directory: {}",
                submission_path.display()
            ))
            .into());
        }

        // Check if reproduce.sh exists
        let reproduce_script = submission_path.join("reproduce.sh");
        if !reproduce_script.exists() {
            return Err(ExecutionError::SandboxFailed(format!(
                "reproduce.sh not found in submission directory: {}",
                submission_path.display()
            ))
            .into());
        }

        // Validate Docker is available
        if !Self::check_docker_available().await {
            return Err(ExecutionError::SandboxFailed(
                "Docker is not available. Please ensure Docker is installed and running."
                    .to_string(),
            )
            .into());
        }

        // Validate Docker image exists (optional check, can be slow)
        // Uncomment if you want to validate image before execution:
        // self.validate_image().await?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&reproduce_script)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&reproduce_script, perms)?;
        }

        let start = std::time::Instant::now();

        // Build docker run command with security hardening
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm") // Remove container after execution
            .arg("--read-only") // Mount root filesystem as read-only for security
            .arg("--tmpfs") // Use tmpfs for /tmp
            .arg("/tmp:rw,noexec,nosuid,size=1g")
            .arg("-v")
            .arg(format!("{}:/workspace:ro", submission_path.display())); // Mount as read-only

        // Add resource limits if specified
        if let Some(memory_mb) = self.memory_limit_mb {
            cmd.arg("-m").arg(format!("{}m", memory_mb));
        }
        if let Some(cpu_limit) = self.cpu_limit {
            cmd.arg("--cpus").arg(format!("{}", cpu_limit));
        }

        // Add security options
        cmd.arg("--security-opt").arg("no-new-privileges:true");

        // Drop all capabilities by default for security
        // Note: Some scripts may need specific capabilities, adjust as needed
        cmd.arg("--cap-drop").arg("ALL");

        // Add network isolation
        cmd.arg("--network").arg("none");

        // Set working directory and image
        cmd.arg("-w").arg("/workspace");
        cmd.arg(&self.docker_image);

        // Execute reproduce.sh
        cmd.arg("bash").arg("reproduce.sh");

        // Execute with timeout
        let output = timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| ExecutionError::Timeout)?;

        let output = output.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to execute Docker container: {}", e))
        })?;

        let duration = start.elapsed();

        // List output files (files that were created/modified)
        let output_files = self.list_output_files(submission_path).await?;

        Ok(ReproductionResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
            output_files,
        })
    }

    async fn list_output_files(&self, submission_path: &std::path::Path) -> Result<Vec<PathBuf>> {
        // List common output file patterns
        let mut output_files = Vec::new();

        // Common output file extensions
        let output_extensions: Vec<&str> = vec![
            "png", "jpg", "jpeg", "pdf", "csv", "json", "txt", "log", "html", "svg", "png", "gif",
            "npy", "npz", "h5", "hdf5", "pkl", "pickle",
        ];

        // Common output directory names
        let output_dirs: Vec<&str> = vec!["output", "results", "figures", "plots", "data", "logs"];

        // Recursively walk the directory
        let mut entries = tokio::fs::read_dir(submission_path).await.map_err(|e| {
            ExecutionError::SandboxFailed(format!("Failed to read submission directory: {}", e))
        })?;

        let mut dirs_to_process = vec![submission_path.to_path_buf()];

        while let Some(dir) = dirs_to_process.pop() {
            let mut current_entries = tokio::fs::read_dir(&dir).await.map_err(|e| {
                ExecutionError::SandboxFailed(format!(
                    "Failed to read directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;

            while let Ok(Some(entry)) = current_entries.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| {
                    ExecutionError::SandboxFailed(format!(
                        "Failed to get metadata for {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                if metadata.is_dir() {
                    // Check if it's an output directory
                    if let Some(dir_name) = path.file_name().and_then(OsStr::to_str) {
                        if output_dirs.contains(&dir_name) {
                            dirs_to_process.push(path.clone());
                        }
                    }
                } else if metadata.is_file() {
                    // Check if file has output extension
                    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
                        if output_extensions.contains(&ext.to_lowercase().as_str()) {
                            output_files.push(path);
                        }
                    }
                }
            }
        }

        // Remove duplicates and sort
        output_files.sort();
        output_files.dedup();

        Ok(output_files)
    }
}

impl Default for ReproductionSystem {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            docker_image: "python:3.11".to_string(),
            timeout: Duration::from_secs(3600),
            memory_limit_mb: Some(4096),
            cpu_limit: Some(2.0),
        })
    }
}
