use crate::config::Config;
use crate::error::{ConfigError, Result};

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate the entire configuration
    pub fn validate(&self, config: &Config) -> Result<()> {
        self.validate_llm_config(&config.llm)?;
        self.validate_agent_config(&config.agents)?;
        self.validate_execution_config(&config.execution)?;
        self.validate_ui_config(&config.ui)?;
        Ok(())
    }

    fn validate_llm_config(&self, llm: &crate::config::LLMConfig) -> Result<()> {
        // Validate default provider exists
        if !llm.providers.contains_key(&llm.default_provider) {
            return Err(ConfigError::Invalid(format!(
                "Default provider '{}' not found in providers",
                llm.default_provider
            ))
            .into());
        }

        // Validate default provider is enabled
        if let Some(provider) = llm.providers.get(&llm.default_provider) {
            if !provider.enabled {
                return Err(ConfigError::Invalid(format!(
                    "Default provider '{}' is disabled",
                    llm.default_provider
                ))
                .into());
            }
        }

        // Validate timeout
        if llm.timeout_seconds == 0 {
            return Err(ConfigError::Invalid(
                "LLM timeout_seconds must be greater than 0".to_string(),
            )
            .into());
        }

        // Validate max retries
        if llm.max_retries == 0 {
            return Err(
                ConfigError::Invalid("LLM max_retries must be greater than 0".to_string()).into(),
            );
        }

        // Validate each provider
        for (name, provider) in &llm.providers {
            if provider.enabled {
                if provider.models.is_empty() {
                    return Err(ConfigError::Invalid(format!(
                        "Provider '{}' has no models configured",
                        name
                    ))
                    .into());
                }

                // Validate temperature
                if provider.temperature < 0.0 || provider.temperature > 2.0 {
                    return Err(ConfigError::Invalid(format!(
                        "Provider '{}' has invalid temperature: {} (must be between 0.0 and 2.0)",
                        name, provider.temperature
                    ))
                    .into());
                }

                // Validate max_tokens if set
                if let Some(max_tokens) = provider.max_tokens {
                    if max_tokens == 0 {
                        return Err(ConfigError::Invalid(format!(
                            "Provider '{}' has invalid max_tokens: {} (must be greater than 0)",
                            name, max_tokens
                        ))
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_agent_config(&self, agents: &crate::config::AgentConfig) -> Result<()> {
        // Validate models are not empty
        if agents.planning_model.is_empty() {
            return Err(ConfigError::Invalid("Planning model cannot be empty".to_string()).into());
        }
        if agents.analysis_model.is_empty() {
            return Err(ConfigError::Invalid("Analysis model cannot be empty".to_string()).into());
        }
        if agents.coding_model.is_empty() {
            return Err(ConfigError::Invalid("Coding model cannot be empty".to_string()).into());
        }
        if agents.verification_model.is_empty() {
            return Err(
                ConfigError::Invalid("Verification model cannot be empty".to_string()).into(),
            );
        }

        // Validate max_iterations
        if agents.max_iterations == 0 {
            return Err(
                ConfigError::Invalid("Max iterations must be greater than 0".to_string()).into(),
            );
        }

        // Validate parallel_tasks
        if agents.parallel_tasks == 0 {
            return Err(
                ConfigError::Invalid("Parallel tasks must be greater than 0".to_string()).into(),
            );
        }

        // Validate task_timeout
        if agents.task_timeout_seconds == 0 {
            return Err(
                ConfigError::Invalid("Task timeout must be greater than 0".to_string()).into(),
            );
        }

        Ok(())
    }

    fn validate_execution_config(&self, execution: &crate::config::ExecutionConfig) -> Result<()> {
        // Validate sandbox_type
        let valid_sandbox_types = ["docker", "isolated", "none"];
        if !valid_sandbox_types.contains(&execution.sandbox_type.as_str()) {
            return Err(ConfigError::Invalid(format!(
                "Invalid sandbox_type: {} (must be one of: {})",
                execution.sandbox_type,
                valid_sandbox_types.join(", ")
            ))
            .into());
        }

        // Validate timeout
        if execution.timeout_seconds == 0 {
            return Err(ConfigError::Invalid(
                "Execution timeout must be greater than 0".to_string(),
            )
            .into());
        }

        // Validate memory limit
        if execution.memory_limit_mb == 0 {
            return Err(
                ConfigError::Invalid("Memory limit must be greater than 0".to_string()).into(),
            );
        }

        // Validate CPU limit
        if execution.cpu_limit <= 0.0 {
            return Err(
                ConfigError::Invalid("CPU limit must be greater than 0".to_string()).into(),
            );
        }

        Ok(())
    }

    fn validate_ui_config(&self, ui: &crate::config::UIConfig) -> Result<()> {
        // Validate refresh_rate_ms
        if ui.refresh_rate_ms == 0 {
            return Err(
                ConfigError::Invalid("UI refresh rate must be greater than 0".to_string()).into(),
            );
        }

        // Validate log_level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&ui.log_level.to_lowercase().as_str()) {
            return Err(ConfigError::Invalid(format!(
                "Invalid log_level: {} (must be one of: {})",
                ui.log_level,
                valid_log_levels.join(", ")
            ))
            .into());
        }

        Ok(())
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}
