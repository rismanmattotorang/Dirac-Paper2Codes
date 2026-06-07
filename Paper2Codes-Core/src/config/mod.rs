pub mod builder;
pub mod loader;
pub mod validation;

pub use loader::ConfigLoader;
pub use validation::ConfigValidator;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::{ConfigError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LLMConfig,
    pub agents: AgentConfig,
    pub verification: VerificationConfig,
    pub execution: ExecutionConfig,
    pub ui: UIConfig,
    pub paths: PathConfig,
    #[serde(default = "StorageConfig::default")]
    pub storage: StorageConfig,
    #[serde(default = "ApiConfig::default")]
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

fn default_retry_delay_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub models: Vec<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: Option<u32>,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> Option<u32> {
    Some(4096)
}

/// Static metadata describing a built-in LLM provider.
///
/// Centralising this information removes the duplicated provider literals that
/// previously lived in `Config::default`, `ConfigLoader::apply_env_overrides`
/// and the settings handlers, and gives the API a single source of truth for
/// display names, default endpoints, model lists and key validation hints.
#[derive(Debug, Clone, Copy)]
pub struct ProviderInfo {
    /// Canonical provider id used as the map key (e.g. `"openai"`).
    pub id: &'static str,
    /// Human friendly name shown in the UI.
    pub display_name: &'static str,
    /// Default API base URL.
    pub base_url: &'static str,
    /// Default model identifiers offered for the provider.
    pub default_models: &'static [&'static str],
    /// Recognised API key prefixes, used for soft client-side validation hints.
    pub key_prefixes: &'static [&'static str],
    /// Environment variable that supplies the key when not set in config.
    pub env_var: &'static str,
}

/// Catalog of providers Paper2Codes knows how to talk to.
pub const KNOWN_PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "openai",
        display_name: "OpenAI",
        base_url: "https://api.openai.com/v1",
        default_models: &["gpt-4-turbo-preview", "gpt-4", "gpt-3.5-turbo"],
        key_prefixes: &["sk-"],
        env_var: "OPENAI_API_KEY",
    },
    ProviderInfo {
        id: "anthropic",
        display_name: "Anthropic",
        base_url: "https://api.anthropic.com/v1",
        default_models: &["claude-3-opus-20240229", "claude-3-sonnet-20240229"],
        key_prefixes: &["sk-ant-"],
        env_var: "ANTHROPIC_API_KEY",
    },
    ProviderInfo {
        id: "openrouter",
        display_name: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        default_models: &[
            "openai/gpt-4-turbo",
            "anthropic/claude-3-opus-20240229",
            "x-ai/grok-2-1212",
        ],
        key_prefixes: &["sk-or-"],
        env_var: "OPENROUTER_API_KEY",
    },
    ProviderInfo {
        id: "xai",
        display_name: "xAI (Grok)",
        base_url: "https://api.x.ai/v1",
        default_models: &["grok-2-1212", "grok-beta"],
        key_prefixes: &["xai-"],
        env_var: "XAI_API_KEY",
    },
];

/// Look up static metadata for a known provider id.
pub fn provider_info(id: &str) -> Option<&'static ProviderInfo> {
    KNOWN_PROVIDERS.iter().find(|p| p.id == id)
}

/// Environment variable name that supplies the API key for `provider_id`.
pub fn provider_env_var(provider_id: &str) -> String {
    provider_info(provider_id)
        .map(|info| info.env_var.to_string())
        .unwrap_or_else(|| format!("{}_API_KEY", provider_id.to_uppercase()))
}

impl ProviderConfig {
    /// Build a sensible default provider configuration for a known provider id.
    ///
    /// Unknown providers get an enabled config with empty metadata so a key can
    /// still be attached to them.
    pub fn for_provider(id: &str) -> Self {
        match provider_info(id) {
            Some(info) => Self {
                enabled: true,
                api_key: None,
                base_url: Some(info.base_url.to_string()),
                models: info.default_models.iter().map(|m| m.to_string()).collect(),
                temperature: default_temperature(),
                max_tokens: default_max_tokens(),
            },
            None => Self {
                enabled: true,
                api_key: None,
                base_url: None,
                models: Vec::new(),
                temperature: default_temperature(),
                max_tokens: default_max_tokens(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub planning_model: String,
    pub analysis_model: String,
    pub coding_model: String,
    pub verification_model: String,
    pub max_iterations: u32,
    pub parallel_tasks: usize,
    #[serde(default = "default_task_timeout_seconds")]
    pub task_timeout_seconds: u64,
}

fn default_task_timeout_seconds() -> u64 {
    600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    pub enable_static_checks: bool,
    pub enable_dynamic_tests: bool,
    pub enable_symbolic_verification: bool,
    #[serde(default = "default_strict_mode")]
    pub strict_mode: bool,
}

fn default_strict_mode() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub sandbox_type: String,
    pub timeout_seconds: u64,
    #[serde(default = "default_memory_limit_mb")]
    pub memory_limit_mb: u64,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f32,
}

fn default_memory_limit_mb() -> u64 {
    2048
}

fn default_cpu_limit() -> f32 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub refresh_rate_ms: u64,
    pub show_logs: bool,
    pub log_level: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_enabled")]
    pub enabled: bool,
    #[serde(default = "default_connection_string")]
    pub connection_string: String,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_connection_timeout_seconds")]
    pub connection_timeout_seconds: u64,
    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,
}

fn default_storage_enabled() -> bool {
    true // Enable storage by default so TUI can connect
}

fn default_connection_string() -> String {
    "localhost:8000".to_string() // SurrealDB Rust client expects host:port format
}

fn default_namespace() -> String {
    "paper2codes".to_string()
}

fn default_database() -> String {
    "main".to_string()
}

fn default_max_connections() -> usize {
    10
}

fn default_connection_timeout_seconds() -> u64 {
    30
}

fn default_auto_migrate() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_enabled")]
    pub enabled: bool,
    #[serde(default = "default_api_host")]
    pub host: String,
    #[serde(default = "default_api_port")]
    pub port: u16,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
    #[serde(default = "default_enable_tui")]
    pub enable_tui: bool,
    #[serde(default = "AuthConfig::default")]
    pub auth: AuthConfig,
    #[serde(default = "RateLimitConfig::default")]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub cache: crate::api::cache::CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration: u64, // seconds
    #[serde(default = "default_refresh_expiration")]
    pub refresh_expiration: u64, // seconds
    #[serde(default = "default_enable_csrf")]
    pub enable_csrf: bool,
    #[serde(default = "default_password_min_length")]
    pub password_min_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_general_rate_limit")]
    pub general_requests_per_minute: u32,
    #[serde(default = "default_auth_rate_limit")]
    pub auth_requests_per_minute: u32,
    #[serde(default = "default_upload_rate_limit")]
    pub upload_requests_per_minute: u32,
    #[serde(default = "default_llm_rate_limit")]
    pub llm_requests_per_minute: u32,
}

fn default_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_string())
}

fn default_jwt_expiration() -> u64 {
    900 // 15 minutes
}

fn default_refresh_expiration() -> u64 {
    604800 // 7 days
}

fn default_enable_csrf() -> bool {
    true
}

fn default_password_min_length() -> usize {
    8
}

fn default_general_rate_limit() -> u32 {
    // Increased for development - adjust in production via config
    1000
}

fn default_auth_rate_limit() -> u32 {
    // Increased for development - adjust in production via config
    50
}

fn default_upload_rate_limit() -> u32 {
    // Increased for development - adjust in production via config
    100
}

fn default_llm_rate_limit() -> u32 {
    // Increased for development - adjust in production via config
    200
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt_secret(),
            jwt_expiration: default_jwt_expiration(),
            refresh_expiration: default_refresh_expiration(),
            enable_csrf: default_enable_csrf(),
            password_min_length: default_password_min_length(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            general_requests_per_minute: default_general_rate_limit(),
            auth_requests_per_minute: default_auth_rate_limit(),
            upload_requests_per_minute: default_upload_rate_limit(),
            llm_requests_per_minute: default_llm_rate_limit(),
        }
    }
}

fn default_api_enabled() -> bool {
    false
}

fn default_api_host() -> String {
    "127.0.0.1".to_string()
}

fn default_api_port() -> u16 {
    8080
}

fn default_cors_origins() -> Vec<String> {
    vec![
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
    ]
}

fn default_enable_tui() -> bool {
    true
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8080,
            cors_origins: vec!["http://localhost:3000".to_string()],
            enable_tui: true,
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cache: crate::api::cache::CacheConfig::default(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            enabled: true,                                   // Enable storage by default
            connection_string: "localhost:8000".to_string(), // SurrealDB Rust client expects host:port format
            namespace: "paper2codes".to_string(),
            database: "main".to_string(),
            username: Some("root".to_string()), // Default username
            password: Some("root".to_string()), // Default password
            max_connections: 10,
            connection_timeout_seconds: 30,
            auto_migrate: true,
        }
    }
}

impl Config {
    /// Load configuration with environment variable overrides
    pub fn load() -> Result<Self> {
        ConfigLoader::new().load()
    }

    /// Load configuration from a specific path
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Result<Self> {
        ConfigLoader::new().with_config_path(path).load()
    }

    /// Create a default configuration
    pub fn default() -> Self {
        let mut providers = HashMap::new();

        // OpenAI provider (default)
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                enabled: true,
                api_key: None, // Should be set via environment variable or config file
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

        // Anthropic provider
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                enabled: true,
                api_key: None, // Should be set via environment variable or config file
                base_url: Some("https://api.anthropic.com/v1".to_string()),
                models: vec![
                    "claude-3-opus-20240229".to_string(),
                    "claude-3-sonnet-20240229".to_string(),
                ],
                temperature: 0.7,
                max_tokens: Some(4096),
            },
        );

        // OpenRouter provider (optional)
        providers.insert(
            "openrouter".to_string(),
            ProviderConfig {
                enabled: false, // Disabled by default
                api_key: None,
                base_url: Some("https://openrouter.ai/api/v1".to_string()),
                models: vec![
                    "openai/gpt-4-turbo".to_string(),
                    "anthropic/claude-3-opus-20240229".to_string(),
                    "x-ai/grok-2-1212".to_string(),
                ],
                temperature: 0.7,
                max_tokens: Some(4096),
            },
        );

        Self {
            llm: LLMConfig {
                default_provider: "openai".to_string(),
                providers,
                timeout_seconds: 300,
                max_retries: 3,
                retry_delay_ms: 1000,
            },
            agents: AgentConfig {
                planning_model: "openai/gpt-4-turbo".to_string(),
                analysis_model: "anthropic/claude-3-opus".to_string(),
                coding_model: "openai/gpt-4-turbo".to_string(),
                verification_model: "anthropic/claude-3-opus".to_string(),
                max_iterations: 10,
                parallel_tasks: 3,
                task_timeout_seconds: 600,
            },
            verification: VerificationConfig {
                enable_static_checks: true,
                enable_dynamic_tests: true,
                enable_symbolic_verification: true,
                strict_mode: false,
            },
            execution: ExecutionConfig {
                sandbox_type: "docker".to_string(),
                timeout_seconds: 300,
                memory_limit_mb: 2048,
                cpu_limit: 2.0,
            },
            ui: UIConfig {
                refresh_rate_ms: 100,
                show_logs: true,
                log_level: "info".to_string(),
                theme: "default".to_string(),
            },
            paths: PathConfig {
                config_dir: None,
                cache_dir: None,
                output_dir: None,
            },
            storage: StorageConfig::default(),
            api: ApiConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 8080,
                cors_origins: vec!["http://localhost:3000".to_string()],
                enable_tui: true,
                auth: AuthConfig::default(),
                rate_limit: RateLimitConfig::default(),
                cache: crate::api::cache::CacheConfig::default(),
            },
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        ConfigValidator::new().validate(self)
    }

    /// Get API key for a provider, checking environment variables
    pub fn get_api_key(&self, provider_name: &str) -> Option<String> {
        // First check config
        if let Some(provider) = self.llm.providers.get(provider_name) {
            if let Some(key) = &provider.api_key {
                return Some(key.clone());
            }
        }

        // Then check environment variables (provider-specific name from the catalog).
        std::env::var(provider_env_var(provider_name)).ok()
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> Result<PathBuf> {
        if let Some(dir) = &self.paths.config_dir {
            Ok(dir.clone())
        } else {
            dirs::config_dir()
                .ok_or_else(|| ConfigError::NotFound("config directory".to_string()).into())
                .map(|d| d.join("paper2codes"))
        }
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> Result<PathBuf> {
        if let Some(dir) = &self.paths.cache_dir {
            Ok(dir.clone())
        } else {
            dirs::cache_dir()
                .ok_or_else(|| ConfigError::NotFound("cache directory".to_string()).into())
                .map(|d| d.join("paper2codes"))
        }
    }

    /// Get the output directory path
    pub fn output_dir(&self) -> Result<PathBuf> {
        if let Some(dir) = &self.paths.output_dir {
            Ok(dir.clone())
        } else {
            Ok(PathBuf::from("./output"))
        }
    }
}
