//! WebSocket broadcasting utilities
//!
//! This module provides helper functions for broadcasting messages from various
//! parts of the application.

use crate::api::state::AppState;
use crate::api::websocket::protocol::{channels, WsMessage};
use tracing::debug;

/// Broadcast a task update message
pub async fn broadcast_task_update(
    state: &AppState,
    task_id: &str,
    status: &str,
    progress: f32,
    message: Option<String>,
    error: Option<String>,
) {
    #[cfg(feature = "api")]
    {
        let channel = channels::task(task_id);
        let msg =
            WsMessage::task_update(channel.clone(), task_id, status, progress, message, error);

        state.websocket_manager.broadcast(msg).await;

        debug!(
            task_id = %task_id,
            status = %status,
            progress = progress,
            "Broadcasted task update"
        );
    }
}

/// Broadcast a paper processed message
pub async fn broadcast_paper_processed(
    state: &AppState,
    paper_id: &str,
    status: &str,
    segments_count: usize,
    message: Option<String>,
) {
    #[cfg(feature = "api")]
    {
        let channel = channels::paper(paper_id);
        let msg =
            WsMessage::paper_processed(channel.clone(), paper_id, status, segments_count, message);

        state.websocket_manager.broadcast(msg).await;

        debug!(
            paper_id = %paper_id,
            status = %status,
            segments_count = segments_count,
            "Broadcasted paper processed"
        );
    }
}

/// Broadcast a generation progress message
pub async fn broadcast_generation_progress(
    state: &AppState,
    paper_id: &str,
    repository_id: Option<String>,
    progress: f32,
    current_module: Option<String>,
    modules_completed: usize,
    modules_total: usize,
    message: Option<String>,
) {
    #[cfg(feature = "api")]
    {
        let channel = if let Some(ref repo_id) = repository_id {
            channels::repository(repo_id)
        } else {
            channels::paper(paper_id)
        };

        let msg = WsMessage::generation_progress(
            channel.clone(),
            paper_id,
            repository_id,
            progress,
            current_module,
            modules_completed,
            modules_total,
            message,
        );

        state.websocket_manager.broadcast(msg).await;

        debug!(
            paper_id = %paper_id,
            progress = progress,
            modules_completed = modules_completed,
            modules_total = modules_total,
            "Broadcasted generation progress"
        );
    }
}

/// Broadcast a verification complete message
pub async fn broadcast_verification_complete(
    state: &AppState,
    repository_id: &str,
    passed: bool,
    issues_count: usize,
    message: Option<String>,
) {
    #[cfg(feature = "api")]
    {
        let channel = channels::repository(repository_id);
        let msg = WsMessage::verification_complete(
            channel.clone(),
            repository_id,
            passed,
            issues_count,
            message,
        );

        state.websocket_manager.broadcast(msg).await;

        debug!(
            repository_id = %repository_id,
            passed = passed,
            issues_count = issues_count,
            "Broadcasted verification complete"
        );
    }
}

/// Broadcast an error message to a specific channel
pub async fn broadcast_error(
    state: &AppState,
    channel: &str,
    code: &str,
    message: &str,
    details: Option<serde_json::Value>,
) {
    #[cfg(feature = "api")]
    {
        let msg = WsMessage::error(channel, code, message, details);
        state.websocket_manager.broadcast(msg).await;

        debug!(
            channel = %channel,
            code = %code,
            "Broadcasted error message"
        );
    }
}

/// Broadcast a message to a specific user
pub async fn broadcast_to_user(state: &AppState, user_id: &str, message: WsMessage) {
    #[cfg(feature = "api")]
    {
        state
            .websocket_manager
            .broadcast_to_user(user_id, message)
            .await;

        debug!(
            user_id = %user_id,
            "Broadcasted message to user"
        );
    }
}
