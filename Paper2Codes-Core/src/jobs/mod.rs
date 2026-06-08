//! Durable job queue for long-running work (Phase 2).
//!
//! Paper processing / code generation must not be a fire-and-forget
//! `tokio::spawn` — work would be lost on restart and cannot be retried,
//! tracked, or cancelled. This module provides a persisted-state job queue with:
//!
//! * a [`Job`] record (status, attempts, progress, last error, timestamps),
//! * enqueue / claim / complete / fail (with capped retries + exponential
//!   backoff) / cancel / progress operations,
//! * a [`JobHandler`] trait and a worker loop ([`JobQueue::run_worker`]).
//!
//! The default [`JobQueue`] keeps state in memory (durable for the life of the
//! process and safe across concurrent workers). The [`Job`] record is
//! `Serialize`/`Deserialize` so the same model maps directly onto a SurrealDB
//! table for cross-restart durability — the production step is to back the
//! queue with that store.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;

/// Lifecycle state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A unit of durable work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    /// Logical job type (e.g. `"paper_generation"`), used to route to a handler.
    pub kind: String,
    /// Opaque payload (typically JSON) for the handler.
    pub payload: String,
    pub status: JobStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    /// Progress in `[0, 1]`.
    pub progress: f32,
    pub last_error: Option<String>,
    /// Earliest time the job may be (re)claimed; `None` = immediately.
    pub ready_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Job {
    fn new(kind: String, payload: String, max_attempts: u32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            kind,
            payload,
            status: JobStatus::Pending,
            attempts: 0,
            max_attempts: max_attempts.max(1),
            progress: 0.0,
            last_error: None,
            ready_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn is_claimable(&self, now: DateTime<Utc>) -> bool {
        self.status == JobStatus::Pending && self.ready_at.map(|t| t <= now).unwrap_or(true)
    }
}

#[derive(Default)]
struct QueueState {
    jobs: HashMap<Uuid, Job>,
    order: Vec<Uuid>,
}

/// An in-memory, concurrency-safe durable job queue.
pub struct JobQueue {
    state: RwLock<QueueState>,
    base_backoff_ms: u64,
    max_backoff_ms: u64,
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self::with_backoff(500, 60_000)
    }

    pub fn with_backoff(base_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            state: RwLock::new(QueueState::default()),
            base_backoff_ms,
            max_backoff_ms,
        }
    }

    /// Enqueue a new job, returning its id.
    pub async fn enqueue(&self, kind: impl Into<String>, payload: impl Into<String>, max_attempts: u32) -> Uuid {
        let job = Job::new(kind.into(), payload.into(), max_attempts);
        let id = job.id;
        let mut state = self.state.write().await;
        state.order.push(id);
        state.jobs.insert(id, job);
        id
    }

    /// Claim the next ready pending job (FIFO), marking it `Running`.
    pub async fn claim_next(&self) -> Option<Job> {
        let now = Utc::now();
        let mut state = self.state.write().await;
        let ids: Vec<Uuid> = state.order.clone();
        for id in ids {
            if let Some(job) = state.jobs.get(&id) {
                if job.is_claimable(now) {
                    let job = state.jobs.get_mut(&id).unwrap();
                    job.status = JobStatus::Running;
                    job.attempts += 1;
                    job.ready_at = None;
                    job.updated_at = now;
                    return Some(job.clone());
                }
            }
        }
        None
    }

    /// Mark a running job complete.
    pub async fn complete(&self, id: Uuid) {
        let mut state = self.state.write().await;
        if let Some(job) = state.jobs.get_mut(&id) {
            job.status = JobStatus::Completed;
            job.progress = 1.0;
            job.updated_at = Utc::now();
        }
    }

    /// Record a failure. Re-queues with exponential backoff while attempts
    /// remain; otherwise marks the job `Failed`. Returns `true` if it will retry.
    pub async fn fail(&self, id: Uuid, error: impl Into<String>) -> bool {
        let mut state = self.state.write().await;
        let (base, max) = (self.base_backoff_ms, self.max_backoff_ms);
        if let Some(job) = state.jobs.get_mut(&id) {
            job.last_error = Some(error.into());
            job.updated_at = Utc::now();
            if job.attempts < job.max_attempts {
                let shift = job.attempts.saturating_sub(1).min(20);
                let backoff = base.saturating_mul(1u64 << shift).min(max);
                job.ready_at = Some(Utc::now() + chrono::Duration::milliseconds(backoff as i64));
                job.status = JobStatus::Pending;
                return true;
            }
            job.status = JobStatus::Failed;
        }
        false
    }

    /// Cancel a pending or running job.
    pub async fn cancel(&self, id: Uuid) -> bool {
        let mut state = self.state.write().await;
        if let Some(job) = state.jobs.get_mut(&id) {
            if matches!(job.status, JobStatus::Pending | JobStatus::Running) {
                job.status = JobStatus::Cancelled;
                job.updated_at = Utc::now();
                return true;
            }
        }
        false
    }

    /// Update a running job's progress (clamped to `[0, 1]`).
    pub async fn set_progress(&self, id: Uuid, progress: f32) {
        let mut state = self.state.write().await;
        if let Some(job) = state.jobs.get_mut(&id) {
            job.progress = progress.clamp(0.0, 1.0);
            job.updated_at = Utc::now();
        }
    }

    pub async fn get(&self, id: Uuid) -> Option<Job> {
        self.state.read().await.jobs.get(&id).cloned()
    }

    /// All jobs in enqueue order.
    pub async fn list(&self) -> Vec<Job> {
        let state = self.state.read().await;
        state
            .order
            .iter()
            .filter_map(|id| state.jobs.get(id).cloned())
            .collect()
    }

    /// Count of jobs still pending or running (i.e. outstanding work).
    pub async fn outstanding(&self) -> usize {
        self.state
            .read()
            .await
            .jobs
            .values()
            .filter(|j| matches!(j.status, JobStatus::Pending | JobStatus::Running))
            .count()
    }

    /// Run a worker loop: claim ready jobs and dispatch them to `handler`,
    /// updating status on success/failure, until `stop` is set. Intended to be
    /// spawned as a background task.
    pub async fn run_worker<H: JobHandler>(
        self: Arc<Self>,
        handler: Arc<H>,
        poll: Duration,
        stop: Arc<AtomicBool>,
    ) {
        while !stop.load(Ordering::Relaxed) {
            match self.claim_next().await {
                Some(job) => match handler.handle(&job).await {
                    Ok(()) => self.complete(job.id).await,
                    Err(e) => {
                        let retrying = self.fail(job.id, e.to_string()).await;
                        tracing::warn!(
                            job = %job.id, kind = %job.kind, attempts = job.attempts,
                            retrying, "job handler failed"
                        );
                    }
                },
                None => tokio::time::sleep(poll).await,
            }
        }
    }
}

/// Executes a job's work. Returning `Err` triggers retry/fail handling.
#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn handle(&self, job: &Job) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn enqueue_claim_complete() {
        let q = JobQueue::new();
        let id = q.enqueue("test", "{}", 3).await;
        let claimed = q.claim_next().await.unwrap();
        assert_eq!(claimed.id, id);
        assert_eq!(claimed.status, JobStatus::Running);
        assert_eq!(claimed.attempts, 1);
        // Nothing else claimable.
        assert!(q.claim_next().await.is_none());
        q.complete(id).await;
        assert_eq!(q.get(id).await.unwrap().status, JobStatus::Completed);
        assert_eq!(q.outstanding().await, 0);
    }

    #[tokio::test]
    async fn retries_then_fails_after_max_attempts() {
        let q = JobQueue::with_backoff(0, 0); // immediately re-claimable
        let id = q.enqueue("test", "{}", 2).await;

        let j = q.claim_next().await.unwrap();
        assert_eq!(j.attempts, 1);
        assert!(q.fail(id, "boom").await, "should retry (1 < 2)");
        assert_eq!(q.get(id).await.unwrap().status, JobStatus::Pending);

        let j = q.claim_next().await.unwrap();
        assert_eq!(j.attempts, 2);
        assert!(!q.fail(id, "boom again").await, "should not retry (2 == 2)");
        let final_job = q.get(id).await.unwrap();
        assert_eq!(final_job.status, JobStatus::Failed);
        assert_eq!(final_job.last_error.as_deref(), Some("boom again"));
    }

    #[tokio::test]
    async fn cancel_prevents_claim() {
        let q = JobQueue::new();
        let id = q.enqueue("test", "{}", 1).await;
        assert!(q.cancel(id).await);
        assert!(q.claim_next().await.is_none());
        assert_eq!(q.get(id).await.unwrap().status, JobStatus::Cancelled);
    }

    #[tokio::test]
    async fn progress_is_clamped() {
        let q = JobQueue::new();
        let id = q.enqueue("test", "{}", 1).await;
        q.set_progress(id, 1.5).await;
        assert_eq!(q.get(id).await.unwrap().progress, 1.0);
    }

    struct CountingHandler {
        seen: AtomicUsize,
        fail_kind: String,
    }
    #[async_trait]
    impl JobHandler for CountingHandler {
        async fn handle(&self, job: &Job) -> Result<()> {
            self.seen.fetch_add(1, Ordering::SeqCst);
            if job.kind == self.fail_kind {
                Err(crate::error::Paper2CodesError::Validation("nope".into()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn worker_drains_queue() {
        let q = Arc::new(JobQueue::with_backoff(0, 0));
        let ok = q.enqueue("ok", "{}", 1).await;
        let bad = q.enqueue("bad", "{}", 1).await; // 1 attempt -> fails, no retry

        let handler = Arc::new(CountingHandler {
            seen: AtomicUsize::new(0),
            fail_kind: "bad".to_string(),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let worker = tokio::spawn(JobQueue::run_worker(
            q.clone(),
            handler.clone(),
            Duration::from_millis(5),
            stop.clone(),
        ));

        // Wait until no work remains (with a timeout).
        for _ in 0..200 {
            if q.outstanding().await == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        stop.store(true, Ordering::Relaxed);
        let _ = worker.await;

        assert_eq!(q.get(ok).await.unwrap().status, JobStatus::Completed);
        assert_eq!(q.get(bad).await.unwrap().status, JobStatus::Failed);
        assert!(handler.seen.load(Ordering::SeqCst) >= 2);
    }
}
