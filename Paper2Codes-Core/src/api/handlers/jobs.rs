//! Durable job inspection endpoints.
//!
//! Now that paper generation runs through the durable [`JobQueue`], the Web UI
//! needs to observe a run's real state (status, progress, retries, errors) and
//! cancel it. These endpoints expose the queue read-only (plus cancel), with
//! the originating `paper_id` parsed out of `paper_generation` payloads so the
//! UI can bind a job to its paper.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::handlers::papers_phase3::PaperJobPayload;
use crate::api::state::AppState;
use crate::api::types::errors::ApiError as ApiErrorResponse;
use crate::api::types::responses::ApiResponse;
use crate::jobs::Job;

/// Public, UI-facing view of a durable job.
#[derive(Debug, Serialize)]
pub struct JobInfo {
    pub id: String,
    pub kind: String,
    /// Paper this job generates code for (for `paper_generation` jobs).
    pub paper_id: Option<String>,
    pub status: String,
    pub progress: f32,
    pub attempts: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Job> for JobInfo {
    fn from(job: &Job) -> Self {
        let paper_id = if job.kind == "paper_generation" {
            serde_json::from_str::<PaperJobPayload>(&job.payload)
                .ok()
                .map(|p| p.paper_id)
        } else {
            None
        };
        Self {
            id: job.id.to_string(),
            kind: job.kind.clone(),
            paper_id,
            status: format!("{:?}", job.status).to_lowercase(),
            progress: job.progress,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            last_error: job.last_error.clone(),
            created_at: job.created_at.to_rfc3339(),
            updated_at: job.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JobListQuery {
    /// Filter to jobs for a given paper.
    pub paper_id: Option<String>,
    /// Filter by status (e.g. `pending`, `running`, `completed`, `failed`).
    pub status: Option<String>,
}

/// List jobs, most-recent first, with optional `paper_id`/`status` filters.
pub async fn list_jobs(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<JobListQuery>,
) -> Json<ApiResponse<Vec<JobInfo>>> {
    let mut jobs: Vec<JobInfo> = state
        .job_queue
        .list()
        .await
        .iter()
        .map(JobInfo::from)
        .collect();

    if let Some(paper_id) = query.paper_id.as_deref() {
        jobs.retain(|j| j.paper_id.as_deref() == Some(paper_id));
    }
    if let Some(status) = query.status.as_deref() {
        let status = status.to_lowercase();
        jobs.retain(|j| j.status == status);
    }

    // Most recent first.
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(ApiResponse::success(jobs))
}

/// Fetch a single job by id.
pub async fn get_job(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<JobInfo>>, (StatusCode, Json<ApiErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::new("INVALID_ID", "Invalid job id")),
        )
    })?;

    match state.job_queue.get(uuid).await {
        Some(job) => Ok(Json(ApiResponse::success(JobInfo::from(&job)))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiErrorResponse::new("NOT_FOUND", "Job not found")),
        )),
    }
}

/// Cancel a pending or running job.
pub async fn cancel_job(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<JobInfo>>, (StatusCode, Json<ApiErrorResponse>)> {
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorResponse::new("INVALID_ID", "Invalid job id")),
        )
    })?;

    let cancelled = state.job_queue.cancel(uuid).await;
    match state.job_queue.get(uuid).await {
        Some(job) if cancelled => Ok(Json(ApiResponse::success(JobInfo::from(&job)))),
        Some(_) => Err((
            StatusCode::CONFLICT,
            Json(ApiErrorResponse::new(
                "NOT_CANCELLABLE",
                "Job is not in a cancellable state",
            )),
        )),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiErrorResponse::new("NOT_FOUND", "Job not found")),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::JobStatus;
    use chrono::Utc;

    fn job(kind: &str, payload: &str, status: JobStatus) -> Job {
        let now = Utc::now();
        Job {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            payload: payload.to_string(),
            status,
            attempts: 1,
            max_attempts: 2,
            progress: 0.5,
            last_error: None,
            ready_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn parses_paper_id_for_generation_jobs() {
        let payload = r#"{"paper_id":"paper-123","skill_id":"quantum","language":"python"}"#;
        let info = JobInfo::from(&job("paper_generation", payload, JobStatus::Running));
        assert_eq!(info.paper_id.as_deref(), Some("paper-123"));
        assert_eq!(info.status, "running"); // lowercased for the API
        assert_eq!(info.max_attempts, 2);
    }

    #[test]
    fn other_kinds_have_no_paper_id() {
        let info = JobInfo::from(&job("other", "{}", JobStatus::Completed));
        assert_eq!(info.paper_id, None);
        assert_eq!(info.status, "completed");
    }
}
