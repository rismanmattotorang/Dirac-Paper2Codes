//! Server-Sent Events (SSE) streaming handlers for Phase 5
//!
//! This module implements streaming endpoints for:
//! - Generation progress streaming
//! - Task logs streaming  
//! - LLM response streaming

use axum::{
    extract::{Extension, Path},
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::Utc;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::api::state::AppState;
use crate::llm::{LLMRequest, LLMRouter, Message, MessageRole};
use crate::types::TaskStatus;

/// Stream generation progress as Server-Sent Events
///
/// Endpoint: GET /api/papers/:id/generate/stream
pub async fn stream_generation_progress(
    Path(paper_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Sse<impl Stream<Item = std::result::Result<Event, axum::Error>>> {
    info!(
        "Starting generation progress stream for paper: {}",
        paper_id
    );

    let stream = stream::unfold(
        (state, paper_id.clone(), false),
        |(state, paper_id, finished)| async move {
            if finished {
                return None;
            }

            let paper_lookup_id = paper_id.clone();
            let tasks = match state
                .with_storage(|storage| {
                    let paper_id = paper_lookup_id.clone();
                    async move { storage.get_tasks_by_paper(&paper_id).await }
                })
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    error!("Failed to load tasks for paper {}: {}", paper_id, e);
                    let event = Event::default().json_data(json!({
                        "paper_id": paper_id,
                        "status": "error",
                        "message": format!("Failed to load tasks: {}", e),
                        "timestamp": Utc::now().to_rfc3339(),
                    }));
                    match event {
                        Ok(event) => return Some((Ok(event), (state, paper_id, true))),
                        Err(err) => {
                            error!("Failed to serialize error event: {}", err);
                            return None;
                        }
                    }
                }
            };

            if tasks.is_empty() {
                let event = Event::default().json_data(json!({
                    "paper_id": paper_id,
                    "status": "pending",
                    "message": "No tasks queued for this paper yet",
                    "progress": 0.0,
                    "timestamp": Utc::now().to_rfc3339(),
                }));
                match event {
                    Ok(event) => {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        return Some((Ok(event), (state, paper_id, false)));
                    }
                    Err(e) => {
                        error!("Failed to serialize pending event: {}", e);
                        return None;
                    }
                }
            }

            let total = tasks.len();
            let completed = tasks
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::Completed))
                .count();
            let failed = tasks
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::Failed(_)))
                .count();
            let in_progress = tasks
                .iter()
                .filter(|task| matches!(task.status, TaskStatus::InProgress))
                .count();

            let status = if failed > 0 {
                "failed"
            } else if completed == total && total > 0 {
                "completed"
            } else if in_progress > 0 {
                "in_progress"
            } else {
                "pending"
            };

            let progress = if total > 0 {
                completed as f32 / total as f32
            } else {
                0.0
            };

            // Build SSE event
            let event_data = json!({
                "paper_id": paper_id,
                "progress": progress,
                "completed_tasks": completed,
                "total_tasks": total,
                "status": status,
                "timestamp": Utc::now().to_rfc3339(),
            });

            let event = match Event::default().json_data(event_data) {
                Ok(e) => e,
                Err(e) => {
                    error!("Failed to serialize SSE event: {}", e);
                    return None;
                }
            };

            // Stop streaming when complete
            if status == "completed" || status == "failed" {
                return Some((Ok(event), (state, paper_id, true)));
            }

            // Wait before next update
            tokio::time::sleep(Duration::from_secs(1)).await;

            Some((Ok(event), (state, paper_id, false)))
        },
    );

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

/// Stream task logs as Server-Sent Events
///
/// Endpoint: GET /api/tasks/:id/logs/stream
pub async fn stream_task_logs(
    Path(task_id): Path<String>,
    Extension(state): Extension<Arc<AppState>>,
) -> Sse<impl Stream<Item = std::result::Result<Event, axum::Error>>> {
    info!("Starting task logs stream for task: {}", task_id);

    let task_id_for_logs = task_id.clone();
    let stream = stream::unfold(
        (state, Uuid::parse_str(&task_id).ok(), None::<String>, false),
        move |(state, task_uuid_opt, last_status, finished)| {
            let task_id = task_id_for_logs.clone();
            async move {
                if finished {
                    return None;
                }

                let task_uuid = match task_uuid_opt {
                    Some(uuid) => uuid,
                    None => {
                        warn!("Invalid task ID format: {}", task_id);
                        let payload = json!({
                            "task_id": task_id,
                            "type": "error",
                            "message": "Invalid task identifier",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        let event = match Event::default().json_data(payload) {
                            Ok(event) => event,
                            Err(err) => {
                                error!("Failed to serialize task error event: {}", err);
                                return None;
                            }
                        };
                        return Some((Ok(event), (state, None, last_status, true)));
                    }
                };

                let lookup_uuid = task_uuid;
                let task_result = state
                    .with_storage(|storage| {
                        let task_id = lookup_uuid;
                        async move { storage.get_task(&task_id).await }
                    })
                    .await;

                let task_opt = match task_result {
                    Ok(task) => task,
                    Err(e) => {
                        error!("Failed to load task {}: {}", task_uuid, e);
                        let payload = json!({
                            "task_id": task_uuid.to_string(),
                            "type": "error",
                            "message": format!("Failed to load task: {}", e),
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        let event = match Event::default().json_data(payload) {
                            Ok(event) => event,
                            Err(err) => {
                                error!("Failed to serialize task error event: {}", err);
                                return None;
                            }
                        };
                        return Some((Ok(event), (state, Some(task_uuid), last_status, true)));
                    }
                };

                let task = match task_opt {
                    Some(task) => task,
                    None => {
                        let payload = json!({
                            "task_id": task_uuid.to_string(),
                            "type": "error",
                            "message": "Task not found",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        let event = match Event::default().json_data(payload) {
                            Ok(event) => event,
                            Err(err) => {
                                error!("Failed to serialize missing-task event: {}", err);
                                return None;
                            }
                        };
                        return Some((Ok(event), (state, Some(task_uuid), last_status, true)));
                    }
                };

                let status_label = match &task.status {
                    TaskStatus::Pending => "pending",
                    TaskStatus::InProgress => "in_progress",
                    TaskStatus::Completed => "completed",
                    TaskStatus::Failed(_) => "failed",
                }
                .to_string();

                let should_emit = match &last_status {
                    Some(previous) => previous != &status_label,
                    None => true,
                };

                let message = match &task.status {
                    TaskStatus::Failed(reason) => Some(reason.clone()),
                    _ => None,
                };

                let event_payload = json!({
                    "task_id": task_uuid.to_string(),
                    "status": status_label,
                    "updated_at": task.updated_at.to_rfc3339(),
                    "message": message,
                });

                let event = match Event::default().json_data(event_payload) {
                    Ok(event) => event,
                    Err(e) => {
                        error!("Failed to serialize SSE log event: {}", e);
                        return None;
                    }
                };

                let completed =
                    matches!(task.status, TaskStatus::Completed | TaskStatus::Failed(_));

                if !completed {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }

                if should_emit {
                    Some((
                        Ok(event),
                        (state, Some(task_uuid), Some(status_label), completed),
                    ))
                } else {
                    let heartbeat = Event::default().event("heartbeat").data("still-running");
                    Some((
                        Ok(heartbeat),
                        (state, Some(task_uuid), last_status, completed),
                    ))
                }
            }
        },
    );

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

/// Stream LLM responses as Server-Sent Events
///
/// Endpoint: GET /api/llm/stream
/// Query params: model, prompt, etc.
pub async fn stream_llm_response(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Extension(state): Extension<Arc<AppState>>,
) -> Sse<impl Stream<Item = std::result::Result<Event, axum::Error>>> {
    let prompt = params
        .get("prompt")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let model = params
        .get("model")
        .cloned()
        .unwrap_or_else(|| state.config.agents.coding_model.clone());

    info!(
        "Starting LLM response stream for model: {}, prompt length: {}",
        model,
        prompt.len()
    );

    fn queue_stream(
        events: Vec<serde_json::Value>,
    ) -> BoxStream<'static, std::result::Result<Event, axum::Error>> {
        stream::unfold(VecDeque::from(events), |mut remaining| async move {
            if let Some(payload) = remaining.pop_front() {
                match Event::default().json_data(payload) {
                    Ok(event) => Some((Ok(event), remaining)),
                    Err(err) => {
                        error!("Failed to serialize LLM SSE event: {}", err);
                        None
                    }
                }
            } else {
                None
            }
        })
        .boxed()
    }

    let keep_alive = KeepAlive::new()
        .interval(Duration::from_secs(15))
        .text("keep-alive-text");

    if prompt.is_empty() {
        let stream = queue_stream(vec![json!({
            "type": "error",
            "message": "Query parameter 'prompt' is required",
            "timestamp": Utc::now().to_rfc3339(),
        })]);
        return Sse::new(stream).keep_alive(keep_alive);
    }

    let router = match LLMRouter::new((*state.config).clone()) {
        Ok(router) => router,
        Err(e) => {
            error!("Failed to initialize LLM router: {}", e);
            let stream = queue_stream(vec![json!({
                "type": "error",
                "message": format!("LLM router unavailable: {}", e),
                "timestamp": Utc::now().to_rfc3339(),
            })]);
            return Sse::new(stream).keep_alive(keep_alive);
        }
    };

    let request = LLMRequest::new(
        vec![Message {
            role: MessageRole::User,
            content: prompt.clone(),
        }],
        model.clone(),
    )
    .with_streaming(true);

    let raw_stream = match router.stream(request.clone(), None).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("LLM streaming request failed: {}", e);
            let stream = queue_stream(vec![json!({
                "type": "error",
                "message": format!("LLM request failed: {}", e),
                "timestamp": Utc::now().to_rfc3339(),
            })]);
            return Sse::new(stream).keep_alive(keep_alive);
        }
    };

    struct LlmSseState {
        stream: Box<dyn futures::Stream<Item = crate::error::Result<String>> + Send + Unpin>,
        index: usize,
        started: bool,
        model: String,
    }

    let stream = stream::unfold(
        Some(LlmSseState {
            stream: raw_stream,
            index: 0,
            started: false,
            model: request.model.clone(),
        }),
        |state_opt| async move {
            let mut state = match state_opt {
                Some(state) => state,
                None => return None,
            };

            if !state.started {
                state.started = true;
                let payload = json!({
                    "type": "start",
                    "model": state.model,
                    "timestamp": Utc::now().to_rfc3339(),
                });
                match Event::default().json_data(payload) {
                    Ok(event) => return Some((Ok(event), Some(state))),
                    Err(err) => {
                        error!("Failed to serialize LLM start event: {}", err);
                        return None;
                    }
                }
            }

            match state.stream.next().await {
                Some(Ok(chunk)) => {
                    let payload = json!({
                        "type": "token",
                        "index": state.index,
                        "content": chunk,
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    state.index += 1;
                    match Event::default().json_data(payload) {
                        Ok(event) => Some((Ok(event), Some(state))),
                        Err(err) => {
                            error!("Failed to serialize LLM token event: {}", err);
                            None
                        }
                    }
                }
                Some(Err(err)) => {
                    let payload = json!({
                        "type": "error",
                        "message": format!("LLM stream error: {}", err),
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    match Event::default().json_data(payload) {
                        Ok(event) => Some((Ok(event), None)),
                        Err(ser_err) => {
                            error!("Failed to serialize LLM error event: {}", ser_err);
                            None
                        }
                    }
                }
                None => {
                    let payload = json!({
                        "type": "done",
                        "timestamp": Utc::now().to_rfc3339(),
                    });
                    match Event::default().json_data(payload) {
                        Ok(event) => Some((Ok(event), None)),
                        Err(err) => {
                            error!("Failed to serialize LLM completion event: {}", err);
                            None
                        }
                    }
                }
            }
        },
    )
    .boxed();

    Sse::new(stream).keep_alive(keep_alive)
}
