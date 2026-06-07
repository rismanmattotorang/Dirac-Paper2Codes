use crate::config::{Config, ProviderConfig};
use crate::error::Result;
use std::path::PathBuf;

/// Builder for creating configurations programmatically
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn with_default_provider(mut self, provider: impl Into<String>) -> Self {
        self.config.llm.default_provider = provider.into();
        self
    }

    pub fn with_llm_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.llm.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_llm_max_retries(mut self, max_retries: u32) -> Self {
        self.config.llm.max_retries = max_retries;
        self
    }

    pub fn add_provider(mut self, name: impl Into<String>, provider: ProviderConfig) -> Self {
        self.config.llm.providers.insert(name.into(), provider);
        self
    }

    pub fn with_planning_model(mut self, model: impl Into<String>) -> Self {
        self.config.agents.planning_model = model.into();
        self
    }

    pub fn with_analysis_model(mut self, model: impl Into<String>) -> Self {
        self.config.agents.analysis_model = model.into();
        self
    }

    pub fn with_coding_model(mut self, model: impl Into<String>) -> Self {
        self.config.agents.coding_model = model.into();
        self
    }

    pub fn with_verification_model(mut self, model: impl Into<String>) -> Self {
        self.config.agents.verification_model = model.into();
        self
    }

    pub fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.config.agents.max_iterations = max_iterations;
        self
    }

    pub fn with_parallel_tasks(mut self, parallel_tasks: usize) -> Self {
        self.config.agents.parallel_tasks = parallel_tasks;
        self
    }

    pub fn with_sandbox_type(mut self, sandbox_type: impl Into<String>) -> Self {
        self.config.execution.sandbox_type = sandbox_type.into();
        self
    }

    pub fn with_execution_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.execution.timeout_seconds = timeout_seconds;
        self
    }

    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.paths.output_dir = Some(dir.into());
        self
    }

    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.config.ui.log_level = level.into();
        self
    }

    /// Build and validate the configuration
    pub fn build(self) -> Result<Config> {
        self.config.validate()?;
        Ok(self.config)
    }

    /// Build without validation (use with caution)
    pub fn build_unchecked(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
