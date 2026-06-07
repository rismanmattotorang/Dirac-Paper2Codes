pub mod context;
pub mod recovery;

#[cfg(feature = "api")]
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// Main error type for Paper2Codes
#[derive(Debug, Error)]
pub enum Paper2CodesError {
    #[error("Document processing error: {0}")]
    Document(#[from] DocumentError),

    #[error("LLM API error: {0}")]
    LLM(#[from] LLMError),

    #[error("Agent execution error: {0}")]
    Agent(#[from] AgentError),

    #[error("Verification error: {0}")]
    Verification(#[from] VerificationError),

    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    #[error("Coordinator error: {0}")]
    Coordinator(String),

    #[error("Retrieval error: {0}")]
    Retrieval(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::errors::StorageError),

    #[error("SurrealDB error: {0}")]
    SurrealDb(String),
}

impl Paper2CodesError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Paper2CodesError::LLM(LLMError::RateLimit) => true,
            Paper2CodesError::LLM(LLMError::Network(_)) => true,
            Paper2CodesError::LLM(LLMError::RequestFailed(_)) => true,
            Paper2CodesError::Execution(ExecutionError::Timeout) => true,
            Paper2CodesError::Io(_) => true,
            _ => false,
        }
    }

    /// Get a user-friendly error message with suggestions
    pub fn user_message(&self) -> String {
        match self {
            Paper2CodesError::Config(ConfigError::MissingField(field)) => {
                format!("Missing required configuration field: {}. Please check your config file or set the appropriate environment variable.", field)
            }
            Paper2CodesError::Config(ConfigError::Invalid(msg)) => {
                format!("Invalid configuration: {}. Please check your config file.", msg)
            }
            Paper2CodesError::LLM(LLMError::RateLimit) => {
                "Rate limit exceeded. Please wait a moment and try again, or reduce the number of parallel requests.".to_string()
            }
            Paper2CodesError::LLM(LLMError::Authentication(msg)) => {
                format!("Authentication failed: {}. Please check your API key.", msg)
            }
            Paper2CodesError::LLM(LLMError::ModelUnavailable(model)) => {
                format!("Model '{}' is not available. Please check your configuration or try a different model.", model)
            }
            Paper2CodesError::Execution(ExecutionError::Timeout) => {
                "Execution timeout. The code took too long to run. Consider increasing the timeout or optimizing the code.".to_string()
            }
            Paper2CodesError::Execution(ExecutionError::ResourceLimit) => {
                "Resource limit exceeded. The code used too much memory or CPU. Consider increasing limits or optimizing the code.".to_string()
            }
            _ => format!("{}", self),
        }
    }

    /// Get error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Paper2CodesError::Config(ConfigError::MissingField(_)) => ErrorSeverity::Critical,
            Paper2CodesError::LLM(LLMError::Authentication(_)) => ErrorSeverity::Critical,
            Paper2CodesError::Execution(ExecutionError::ResourceLimit) => ErrorSeverity::Critical,
            Paper2CodesError::LLM(LLMError::RateLimit) => ErrorSeverity::Warning,
            Paper2CodesError::Execution(ExecutionError::Timeout) => ErrorSeverity::Warning,
            _ => ErrorSeverity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("Failed to parse PDF: {0}")]
    ParseFailed(String),

    #[error("Unsupported document format: {0}")]
    UnsupportedFormat(String),

    #[error("Domain classification failed: {0}")]
    ClassificationFailed(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid document structure: {0}")]
    InvalidStructure(String),
}

#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Model not available: {0}")]
    ModelUnavailable(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Timeout: request took too long")]
    Timeout,

    #[error("Invalid request parameters: {0}")]
    InvalidRequest(String),
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid task type for agent: {0}")]
    InvalidTaskType(String),

    #[error("Agent timeout: {0}")]
    Timeout(String),

    #[error("Agent initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Agent context missing: {0}")]
    MissingContext(String),
}

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Static analysis failed: {0}")]
    StaticAnalysis(String),

    #[error("Test execution failed: {0}")]
    TestExecution(String),

    #[error("Symbolic verification failed: {0}")]
    SymbolicVerification(String),

    #[error("Verification tool not available: {0}")]
    ToolUnavailable(String),

    #[error("Invalid specification: {0}")]
    InvalidSpecification(String),
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Sandbox execution failed: {0}")]
    SandboxFailed(String),

    #[error("Code compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Resource limit exceeded")]
    ResourceLimit,

    #[error("Sandbox not available: {0}")]
    SandboxUnavailable(String),

    #[error("Invalid code: {0}")]
    InvalidCode(String),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}

pub type Result<T> = std::result::Result<T, Paper2CodesError>;

#[cfg(feature = "api")]
impl IntoResponse for Paper2CodesError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            Paper2CodesError::Config(ConfigError::MissingField(_)) => {
                (StatusCode::BAD_REQUEST, "CONFIG_ERROR", self.user_message())
            }
            Paper2CodesError::Config(ConfigError::Invalid(_)) => {
                (StatusCode::BAD_REQUEST, "CONFIG_ERROR", self.user_message())
            }
            Paper2CodesError::LLM(LLMError::RateLimit) => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT",
                self.user_message(),
            ),
            Paper2CodesError::LLM(LLMError::Authentication(_)) => (
                StatusCode::UNAUTHORIZED,
                "AUTHENTICATION_ERROR",
                self.user_message(),
            ),
            Paper2CodesError::LLM(LLMError::ModelUnavailable(_)) => (
                StatusCode::BAD_REQUEST,
                "MODEL_UNAVAILABLE",
                self.user_message(),
            ),
            Paper2CodesError::Execution(ExecutionError::Timeout) => (
                StatusCode::REQUEST_TIMEOUT,
                "EXECUTION_TIMEOUT",
                self.user_message(),
            ),
            Paper2CodesError::Execution(ExecutionError::ResourceLimit) => (
                StatusCode::INSUFFICIENT_STORAGE,
                "RESOURCE_LIMIT",
                self.user_message(),
            ),
            Paper2CodesError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            Paper2CodesError::Storage(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
                format!("{}", self),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                format!("{}", self),
            ),
        };

        let error_body = serde_json::json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        });

        (status, Json(error_body)).into_response()
    }
}
