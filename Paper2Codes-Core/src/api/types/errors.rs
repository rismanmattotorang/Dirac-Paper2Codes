use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// API error response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub success: bool,
    pub error: ErrorDetails,
    pub meta: ResponseMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub timestamp: String,
    pub request_id: String,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            success: false,
            error: ErrorDetails {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            },
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: Uuid::new_v4().to_string(),
            },
        }
    }

    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            success: false,
            error: ErrorDetails {
                code: code.to_string(),
                message: message.to_string(),
                details: Some(details),
            },
            meta: ResponseMeta {
                timestamp: Utc::now().to_rfc3339(),
                request_id: Uuid::new_v4().to_string(),
            },
        }
    }

    #[cfg(feature = "api")]
    pub fn validation_error(message: &str, errors: validator::ValidationErrors) -> Self {
        Self::with_details(
            "VALIDATION_ERROR",
            message,
            serde_json::to_value(errors).unwrap_or(serde_json::Value::Null),
        )
    }

    pub fn validation_error_simple(message: &str) -> Self {
        Self::new("VALIDATION_ERROR", message)
    }

    pub fn unauthorized(message: &str) -> Self {
        Self::new("UNAUTHORIZED", message)
    }

    pub fn internal_error(message: &str) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}
