use crate::config::Config;
use crate::error::{ConfigError, Result};
use std::path::{Path, PathBuf};

/// Configuration loader with support for environment variable overrides
pub struct ConfigLoader {
    config_path: Option<PathBuf>,
    use_env_overrides: bool,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            config_path: None,
            use_env_overrides: true,
        }
    }

    /// Set a custom config file path
    pub fn with_config_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Enable or disable environment variable overrides
    pub fn with_env_overrides(mut self, enable: bool) -> Self {
        self.use_env_overrides = enable;
        self
    }

    /// Load configuration from file or create default
    pub fn load(&self) -> Result<Config> {
        let config_path = self.get_config_path()?;

        let mut config = if config_path.exists() {
            self.load_from_file(&config_path)?
        } else {
            // Create default config and save it
            let default_config = Config::default();
            std::fs::create_dir_all(
                config_path
                    .parent()
                    .ok_or_else(|| ConfigError::Invalid("Invalid config path".to_string()))?,
            )?;
            let toml = toml::to_string_pretty(&default_config).map_err(|e| {
                ConfigError::Invalid(format!("Failed to serialize default config: {}", e))
            })?;
            std::fs::write(&config_path, toml)?;
            default_config
        };

        // Apply environment variable overrides
        if self.use_env_overrides {
            self.apply_env_overrides(&mut config)?;
        }

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    fn get_config_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.config_path {
            Ok(path.clone())
        } else {
            let config_dir = dirs::config_dir()
                .ok_or_else(|| ConfigError::NotFound("config directory".to_string()))?
                .join("paper2codes");
            Ok(config_dir.join("config.toml"))
        }
    }

    fn load_from_file(&self, path: &Path) -> Result<Config> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConfigError::Invalid(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            ConfigError::Invalid(format!(
                "Failed to parse config file {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    fn apply_env_overrides(&self, config: &mut Config) -> Result<()> {
        // Override LLM default provider
        if let Ok(provider) = std::env::var("PAPER2CODES_LLM_DEFAULT_PROVIDER") {
            config.llm.default_provider = provider;
        }

        // Override LLM timeout
        if let Ok(timeout) = std::env::var("PAPER2CODES_LLM_TIMEOUT_SECONDS") {
            config.llm.timeout_seconds = timeout.parse().map_err(|_| {
                ConfigError::Invalid("Invalid PAPER2CODES_LLM_TIMEOUT_SECONDS".to_string())
            })?;
        }

        // Override LLM max retries
        if let Ok(retries) = std::env::var("PAPER2CODES_LLM_MAX_RETRIES") {
            config.llm.max_retries = retries.parse().map_err(|_| {
                ConfigError::Invalid("Invalid PAPER2CODES_LLM_MAX_RETRIES".to_string())
            })?;
        }

        // Override agent models
        if let Ok(model) = std::env::var("PAPER2CODES_PLANNING_MODEL") {
            config.agents.planning_model = model;
        }
        if let Ok(model) = std::env::var("PAPER2CODES_ANALYSIS_MODEL") {
            config.agents.analysis_model = model;
        }
        if let Ok(model) = std::env::var("PAPER2CODES_CODING_MODEL") {
            config.agents.coding_model = model;
        }
        if let Ok(model) = std::env::var("PAPER2CODES_VERIFICATION_MODEL") {
            config.agents.verification_model = model;
        }

        // Override agent max iterations
        if let Ok(iterations) = std::env::var("PAPER2CODES_MAX_ITERATIONS") {
            config.agents.max_iterations = iterations.parse().map_err(|_| {
                ConfigError::Invalid("Invalid PAPER2CODES_MAX_ITERATIONS".to_string())
            })?;
        }

        // Override parallel tasks
        if let Ok(tasks) = std::env::var("PAPER2CODES_PARALLEL_TASKS") {
            config.agents.parallel_tasks = tasks.parse().map_err(|_| {
                ConfigError::Invalid("Invalid PAPER2CODES_PARALLEL_TASKS".to_string())
            })?;
        }

        // Override execution timeout
        if let Ok(timeout) = std::env::var("PAPER2CODES_EXECUTION_TIMEOUT_SECONDS") {
            config.execution.timeout_seconds = timeout.parse().map_err(|_| {
                ConfigError::Invalid("Invalid PAPER2CODES_EXECUTION_TIMEOUT_SECONDS".to_string())
            })?;
        }

        // Override UI log level
        if let Ok(level) = std::env::var("PAPER2CODES_LOG_LEVEL") {
            config.ui.log_level = level;
        }

        // Override paths
        if let Ok(dir) = std::env::var("PAPER2CODES_CONFIG_DIR") {
            config.paths.config_dir = Some(PathBuf::from(dir));
        }
        if let Ok(dir) = std::env::var("PAPER2CODES_CACHE_DIR") {
            config.paths.cache_dir = Some(PathBuf::from(dir));
        }
        if let Ok(dir) = std::env::var("PAPER2CODES_OUTPUT_DIR") {
            config.paths.output_dir = Some(PathBuf::from(dir));
        }

        // Override LLM API keys from environment variables
        // OPENROUTER_API_KEY
        if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            if let Some(provider) = config.llm.providers.get_mut("openrouter") {
                provider.api_key = Some(api_key);
            } else {
                // Create openrouter provider if it doesn't exist
                use crate::config::ProviderConfig;
                config.llm.providers.insert(
                    "openrouter".to_string(),
                    ProviderConfig {
                        enabled: true,
                        api_key: Some(std::env::var("OPENROUTER_API_KEY").unwrap()),
                        base_url: Some("https://openrouter.ai/api/v1".to_string()),
                        models: vec![
                            "openai/gpt-4-turbo".to_string(),
                            "anthropic/claude-3-opus-20240229".to_string(),
                        ],
                        temperature: 0.7,
                        max_tokens: Some(4096),
                    },
                );
            }
        }

        // OPENAI_API_KEY
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            if let Some(provider) = config.llm.providers.get_mut("openai") {
                provider.api_key = Some(api_key);
                provider.enabled = true; // Enable if API key is provided
            } else {
                // Create OpenAI provider if it doesn't exist
                use crate::config::ProviderConfig;
                config.llm.providers.insert(
                    "openai".to_string(),
                    ProviderConfig {
                        enabled: true,
                        api_key: Some(api_key),
                        base_url: Some("https://api.openai.com/v1".to_string()),
                        models: vec![
                            "gpt-4-turbo-preview".to_string(),
                            "gpt-4".to_string(),
                            "gpt-3.5-turbo".to_string(),
                        ],
                        temperature: 0.7,
                        max_tokens: Some(4096),
                    },
                );
            }
        }

        // ANTHROPIC_API_KEY
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            if let Some(provider) = config.llm.providers.get_mut("anthropic") {
                provider.api_key = Some(api_key);
                provider.enabled = true; // Enable if API key is provided
            } else {
                // Create Anthropic provider if it doesn't exist
                use crate::config::ProviderConfig;
                config.llm.providers.insert(
                    "anthropic".to_string(),
                    ProviderConfig {
                        enabled: true,
                        api_key: Some(api_key),
                        base_url: Some("https://api.anthropic.com/v1".to_string()),
                        models: vec![
                            "claude-3-opus-20240229".to_string(),
                            "claude-3-sonnet-20240229".to_string(),
                        ],
                        temperature: 0.7,
                        max_tokens: Some(4096),
                    },
                );
            }
        }

        // Override storage configuration from environment variables
        if let Ok(connection_string) = std::env::var("DATABASE_URL") {
            config.storage.connection_string = connection_string;
        }
        if let Ok(enabled) = std::env::var("STORAGE_ENABLED") {
            config.storage.enabled = enabled.parse().unwrap_or(config.storage.enabled);
        }
        if let Ok(namespace) = std::env::var("DATABASE_NAMESPACE") {
            config.storage.namespace = namespace;
        }
        if let Ok(database) = std::env::var("DATABASE_NAME") {
            config.storage.database = database;
        }
        if let Ok(username) = std::env::var("DATABASE_USER") {
            config.storage.username = Some(username);
        }
        if let Ok(password) = std::env::var("DATABASE_PASS") {
            config.storage.password = Some(password);
        }

        // Secret-file convention (Docker/Kubernetes secrets, systemd credentials):
        // a `*_FILE` env var points at a file whose trimmed contents are the
        // secret. Keeps plaintext secrets out of config files and process args.
        Self::apply_secret_files(config);

        Ok(())
    }

    /// Read secrets from files referenced by `*_FILE` environment variables.
    fn apply_secret_files(config: &mut Config) {
        use crate::config::ProviderConfig;

        // JWT secret.
        if let Some(secret) = read_secret_file("JWT_SECRET_FILE") {
            config.api.auth.jwt_secret = secret;
        }
        // Storage password.
        if let Some(secret) = read_secret_file("DATABASE_PASS_FILE") {
            config.storage.password = Some(secret);
        }
        // Per-provider API keys: <PROVIDER>_API_KEY_FILE.
        for info in crate::config::KNOWN_PROVIDERS {
            let var = format!("{}_API_KEY_FILE", info.id.to_uppercase());
            if let Some(key) = read_secret_file(&var) {
                let entry = config
                    .llm
                    .providers
                    .entry(info.id.to_string())
                    .or_insert_with(|| ProviderConfig::for_provider(info.id));
                entry.api_key = Some(key);
                entry.enabled = true;
            }
        }
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Read a secret from the file named by environment variable `var` (the `*_FILE`
/// convention). Returns the trimmed contents, or `None` if the var is unset, the
/// file is unreadable, or the contents are empty.
fn read_secret_file(var: &str) -> Option<String> {
    let path = std::env::var(var).ok()?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let trimmed = contents.trim().to_string();
            if trimmed.is_empty() {
                tracing::warn!("Secret file {} ({}) is empty", var, path);
                None
            } else {
                Some(trimmed)
            }
        }
        Err(e) => {
            tracing::warn!("Failed to read secret file {} ({}): {}", var, path, e);
            None
        }
    }
}

#[cfg(test)]
mod secret_file_tests {
    use super::*;

    #[test]
    fn reads_and_trims_secret_file() {
        let dir = std::env::temp_dir().join(format!("p2c_secret_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("jwt.txt");
        std::fs::write(&path, "  super-secret-value\n").unwrap();

        let var = "P2C_TEST_JWT_SECRET_FILE";
        std::env::set_var(var, &path);
        assert_eq!(read_secret_file(var).as_deref(), Some("super-secret-value"));
        std::env::remove_var(var);

        // Missing var -> None.
        assert_eq!(read_secret_file("P2C_TEST_DEFINITELY_UNSET_FILE"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
