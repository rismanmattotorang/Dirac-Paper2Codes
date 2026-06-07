//! WebSocket message protocol definitions
//!
//! This module defines the message types and protocol for WebSocket communication
//! between the server and clients.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Task status/progress update
    #[serde(rename = "task_update")]
    TaskUpdate {
        channel: String,
        data: TaskUpdateData,
        timestamp: String,
    },

    /// Paper processing complete
    #[serde(rename = "paper_processed")]
    PaperProcessed {
        channel: String,
        data: PaperProcessedData,
        timestamp: String,
    },

    /// Code generation progress
    #[serde(rename = "generation_progress")]
    GenerationProgress {
        channel: String,
        data: GenerationProgressData,
        timestamp: String,
    },

    /// Verification complete
    #[serde(rename = "verification_complete")]
    VerificationComplete {
        channel: String,
        data: VerificationCompleteData,
        timestamp: String,
    },

    /// Error notification
    #[serde(rename = "error")]
    Error {
        channel: String,
        data: ErrorData,
        timestamp: String,
    },

    /// Connection keepalive heartbeat
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: String },

    /// Client subscription request
    #[serde(rename = "subscribe")]
    Subscribe { channels: Vec<String> },

    /// Client unsubscription request
    #[serde(rename = "unsubscribe")]
    Unsubscribe { channels: Vec<String> },

    /// Client subscription confirmation
    #[serde(rename = "subscribed")]
    Subscribed { channels: Vec<String> },

    /// Client unsubscription confirmation
    #[serde(rename = "unsubscribed")]
    Unsubscribed { channels: Vec<String> },
}

/// Task update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdateData {
    pub task_id: String,
    pub status: String,
    pub progress: f32,
    pub message: Option<String>,
    pub error: Option<String>,
}

/// Paper processed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperProcessedData {
    pub paper_id: String,
    pub status: String,
    pub segments_count: usize,
    pub message: Option<String>,
}

/// Generation progress data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgressData {
    pub paper_id: String,
    pub repository_id: Option<String>,
    pub progress: f32,
    pub current_module: Option<String>,
    pub modules_completed: usize,
    pub modules_total: usize,
    pub message: Option<String>,
}

/// Verification complete data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCompleteData {
    pub repository_id: String,
    pub passed: bool,
    pub issues_count: usize,
    pub message: Option<String>,
}

/// Error data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl WsMessage {
    /// Create a task update message
    pub fn task_update(
        channel: impl Into<String>,
        task_id: impl Into<String>,
        status: impl Into<String>,
        progress: f32,
        message: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self::TaskUpdate {
            channel: channel.into(),
            data: TaskUpdateData {
                task_id: task_id.into(),
                status: status.into(),
                progress,
                message,
                error,
            },
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a paper processed message
    pub fn paper_processed(
        channel: impl Into<String>,
        paper_id: impl Into<String>,
        status: impl Into<String>,
        segments_count: usize,
        message: Option<String>,
    ) -> Self {
        Self::PaperProcessed {
            channel: channel.into(),
            data: PaperProcessedData {
                paper_id: paper_id.into(),
                status: status.into(),
                segments_count,
                message,
            },
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a generation progress message
    pub fn generation_progress(
        channel: impl Into<String>,
        paper_id: impl Into<String>,
        repository_id: Option<String>,
        progress: f32,
        current_module: Option<String>,
        modules_completed: usize,
        modules_total: usize,
        message: Option<String>,
    ) -> Self {
        Self::GenerationProgress {
            channel: channel.into(),
            data: GenerationProgressData {
                paper_id: paper_id.into(),
                repository_id,
                progress,
                current_module,
                modules_completed,
                modules_total,
                message,
            },
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a verification complete message
    pub fn verification_complete(
        channel: impl Into<String>,
        repository_id: impl Into<String>,
        passed: bool,
        issues_count: usize,
        message: Option<String>,
    ) -> Self {
        Self::VerificationComplete {
            channel: channel.into(),
            data: VerificationCompleteData {
                repository_id: repository_id.into(),
                passed,
                issues_count,
                message,
            },
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create an error message
    pub fn error(
        channel: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::Error {
            channel: channel.into(),
            data: ErrorData {
                code: code.into(),
                message: message.into(),
                details,
            },
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat() -> Self {
        Self::Heartbeat {
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Get the channel name for this message
    pub fn channel(&self) -> Option<&str> {
        match self {
            Self::TaskUpdate { channel, .. } => Some(channel),
            Self::PaperProcessed { channel, .. } => Some(channel),
            Self::GenerationProgress { channel, .. } => Some(channel),
            Self::VerificationComplete { channel, .. } => Some(channel),
            Self::Error { channel, .. } => Some(channel),
            Self::Heartbeat { .. } => None,
            Self::Subscribe { .. } => None,
            Self::Unsubscribe { .. } => None,
            Self::Subscribed { .. } => None,
            Self::Unsubscribed { .. } => None,
        }
    }
}

/// WebSocket channel names
pub mod channels {
    /// All task updates
    pub const TASKS: &str = "tasks";

    /// Specific task updates (format: "tasks:{task_id}")
    pub fn task(task_id: &str) -> String {
        format!("tasks:{}", task_id)
    }

    /// Paper-specific updates (format: "papers:{paper_id}")
    pub fn paper(paper_id: &str) -> String {
        format!("papers:{}", paper_id)
    }

    /// Repository-specific updates (format: "repos:{repo_id}")
    pub fn repository(repo_id: &str) -> String {
        format!("repos:{}", repo_id)
    }

    /// User-specific updates (format: "user:{user_id}")
    pub fn user(user_id: &str) -> String {
        format!("user:{}", user_id)
    }
}
