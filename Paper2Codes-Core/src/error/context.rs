use std::fmt;
use std::sync::Arc;

/// Error context for providing additional information about errors
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub message: String,
    pub source: Option<Arc<dyn std::error::Error + Send + Sync>>,
    pub context: Vec<String>,
    pub suggestion: Option<String>,
}

impl ErrorContext {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
            context: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_source(mut self, source: Arc<dyn std::error::Error + Send + Sync>) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context.push(context.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn add_context(&mut self, context: impl Into<String>) {
        self.context.push(context.into());
    }

    pub fn format(&self) -> String {
        let mut output = self.message.clone();

        if !self.context.is_empty() {
            output.push_str("\n\nContext:");
            for (i, ctx) in self.context.iter().enumerate() {
                output.push_str(&format!("\n  {}. {}", i + 1, ctx));
            }
        }

        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("\n\nSuggestion: {}", suggestion));
        }

        if let Some(source) = &self.source {
            output.push_str(&format!("\n\nSource: {}", source));
        }

        output
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl std::error::Error for ErrorContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &dyn std::error::Error)
    }
}

/// Extension trait for adding context to errors
pub trait ErrorContextExt {
    fn with_context(self, context: impl Into<String>) -> Self;
    fn with_suggestion(self, suggestion: impl Into<String>) -> Self;
}

impl<T> ErrorContextExt for Result<T, crate::error::Paper2CodesError> {
    fn with_context(self, context: impl Into<String>) -> Self {
        self.map_err(|e| {
            let ctx = context.into();
            // Wrap error messages with context for all error types
            match e {
                crate::error::Paper2CodesError::Config(cfg_err) => {
                    crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(
                        format!("{}: {}", ctx, cfg_err),
                    ))
                }
                crate::error::Paper2CodesError::Document(doc_err) => {
                    crate::error::Paper2CodesError::Document(match doc_err {
                        crate::error::DocumentError::ParseFailed(msg) => {
                            crate::error::DocumentError::ParseFailed(format!("{}: {}", ctx, msg))
                        }
                        crate::error::DocumentError::UnsupportedFormat(msg) => {
                            crate::error::DocumentError::UnsupportedFormat(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::DocumentError::ClassificationFailed(msg) => {
                            crate::error::DocumentError::ClassificationFailed(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::DocumentError::FileNotFound(msg) => {
                            crate::error::DocumentError::FileNotFound(format!("{}: {}", ctx, msg))
                        }
                        crate::error::DocumentError::InvalidStructure(msg) => {
                            crate::error::DocumentError::InvalidStructure(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                    })
                }
                crate::error::Paper2CodesError::LLM(llm_err) => {
                    crate::error::Paper2CodesError::LLM(match llm_err {
                        crate::error::LLMError::RequestFailed(msg) => {
                            crate::error::LLMError::RequestFailed(format!("{}: {}", ctx, msg))
                        }
                        crate::error::LLMError::InvalidResponse(msg) => {
                            crate::error::LLMError::InvalidResponse(format!("{}: {}", ctx, msg))
                        }
                        crate::error::LLMError::ModelUnavailable(msg) => {
                            crate::error::LLMError::ModelUnavailable(format!("{}: {}", ctx, msg))
                        }
                        crate::error::LLMError::Authentication(msg) => {
                            crate::error::LLMError::Authentication(format!("{}: {}", ctx, msg))
                        }
                        crate::error::LLMError::InvalidRequest(msg) => {
                            crate::error::LLMError::InvalidRequest(format!("{}: {}", ctx, msg))
                        }
                        e => e, // RateLimit, Network, Timeout don't have messages
                    })
                }
                crate::error::Paper2CodesError::Agent(agent_err) => {
                    crate::error::Paper2CodesError::Agent(match agent_err {
                        crate::error::AgentError::ExecutionFailed(msg) => {
                            crate::error::AgentError::ExecutionFailed(format!("{}: {}", ctx, msg))
                        }
                        crate::error::AgentError::InvalidTaskType(msg) => {
                            crate::error::AgentError::InvalidTaskType(format!("{}: {}", ctx, msg))
                        }
                        crate::error::AgentError::Timeout(msg) => {
                            crate::error::AgentError::Timeout(format!("{}: {}", ctx, msg))
                        }
                        crate::error::AgentError::InitializationFailed(msg) => {
                            crate::error::AgentError::InitializationFailed(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::AgentError::MissingContext(msg) => {
                            crate::error::AgentError::MissingContext(format!("{}: {}", ctx, msg))
                        }
                    })
                }
                crate::error::Paper2CodesError::Verification(ver_err) => {
                    crate::error::Paper2CodesError::Verification(match ver_err {
                        crate::error::VerificationError::StaticAnalysis(msg) => {
                            crate::error::VerificationError::StaticAnalysis(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::VerificationError::TestExecution(msg) => {
                            crate::error::VerificationError::TestExecution(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::VerificationError::SymbolicVerification(msg) => {
                            crate::error::VerificationError::SymbolicVerification(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::VerificationError::ToolUnavailable(msg) => {
                            crate::error::VerificationError::ToolUnavailable(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::VerificationError::InvalidSpecification(msg) => {
                            crate::error::VerificationError::InvalidSpecification(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                    })
                }
                crate::error::Paper2CodesError::Execution(exec_err) => {
                    crate::error::Paper2CodesError::Execution(match exec_err {
                        crate::error::ExecutionError::SandboxFailed(msg) => {
                            crate::error::ExecutionError::SandboxFailed(format!("{}: {}", ctx, msg))
                        }
                        crate::error::ExecutionError::CompilationFailed(msg) => {
                            crate::error::ExecutionError::CompilationFailed(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::ExecutionError::SandboxUnavailable(msg) => {
                            crate::error::ExecutionError::SandboxUnavailable(format!(
                                "{}: {}",
                                ctx, msg
                            ))
                        }
                        crate::error::ExecutionError::InvalidCode(msg) => {
                            crate::error::ExecutionError::InvalidCode(format!("{}: {}", ctx, msg))
                        }
                        e => e, // Timeout, ResourceLimit don't have messages
                    })
                }
                crate::error::Paper2CodesError::Validation(msg) => {
                    crate::error::Paper2CodesError::Validation(format!("{}: {}", ctx, msg))
                }
                crate::error::Paper2CodesError::Coordinator(msg) => {
                    crate::error::Paper2CodesError::Coordinator(format!("{}: {}", ctx, msg))
                }
                crate::error::Paper2CodesError::Retrieval(msg) => {
                    crate::error::Paper2CodesError::Retrieval(format!("{}: {}", ctx, msg))
                }
                crate::error::Paper2CodesError::SurrealDb(msg) => {
                    crate::error::Paper2CodesError::SurrealDb(format!("{}: {}", ctx, msg))
                }
                e => e, // Io, Serialization, TomlSerialization, TomlDeserialization, Storage
            }
        })
    }

    fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        // For now, append suggestion to error message
        // In a more sophisticated implementation, we could store suggestions separately
        self.map_err(|e| {
            let suggestion_msg = suggestion.into();
            match e {
                crate::error::Paper2CodesError::Config(cfg_err) => {
                    crate::error::Paper2CodesError::Config(match cfg_err {
                        crate::error::ConfigError::Invalid(msg) => {
                            crate::error::ConfigError::Invalid(format!(
                                "{}. Suggestion: {}",
                                msg, suggestion_msg
                            ))
                        }
                        crate::error::ConfigError::ValidationFailed(msg) => {
                            crate::error::ConfigError::ValidationFailed(format!(
                                "{}. Suggestion: {}",
                                msg, suggestion_msg
                            ))
                        }
                        e => e,
                    })
                }
                crate::error::Paper2CodesError::LLM(llm_err) => {
                    crate::error::Paper2CodesError::LLM(match llm_err {
                        crate::error::LLMError::RateLimit => crate::error::LLMError::RequestFailed(
                            format!("Rate limit exceeded. Suggestion: {}", suggestion_msg),
                        ),
                        crate::error::LLMError::RequestFailed(msg) => {
                            crate::error::LLMError::RequestFailed(format!(
                                "{}. Suggestion: {}",
                                msg, suggestion_msg
                            ))
                        }
                        e => e,
                    })
                }
                e => e,
            }
        })
    }
}
