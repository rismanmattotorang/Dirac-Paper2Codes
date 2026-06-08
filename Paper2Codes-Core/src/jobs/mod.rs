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
//! The [`JobQueue`] keeps an in-memory index for fast claiming and concurrency
//! safety, but can be backed by a [`JobStore`] for cross-restart durability.
//! When a store is attached, every state transition is written through to it,
//! [`JobQueue::restore`] reloads outstanding work on boot (resetting jobs that
//! were `Running` when the process died back to `Pending` so no work is lost),
//! and the [`Job`] record's `Serialize`/`Deserialize` maps directly onto a
//! SurrealDB table. With no store attached the queue is in-memory only
//! (durable for the life of the process), which keeps tests and the default
//! path unchanged.

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

/// Durable backing store for the job queue. Implemented by the storage layer
/// (SurrealDB) so jobs survive a process restart; left unset for the in-memory
/// default used by tests and single-process runs.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Upsert the job's current state.
    async fn persist_job(&self, job: &Job) -> Result<()>;
    /// Load all jobs (used once on startup to rebuild the in-memory index).
    async fn load_jobs(&self) -> Result<Vec<Job>>;
}

#[derive(Default)]
struct QueueState {
    jobs: HashMap<Uuid, Job>,
    order: Vec<Uuid>,
}

/// A concurrency-safe job queue with an in-memory index and an optional durable
/// backing [`JobStore`].
pub struct JobQueue {
    state: RwLock<QueueState>,
    base_backoff_ms: u64,
    max_backoff_ms: u64,
    persistence: Option<Arc<dyn JobStore>>,
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
            persistence: None,
        }
    }

    /// Attach a durable backing store. Call [`JobQueue::restore`] afterwards to
    /// rehydrate outstanding work.
    pub fn with_persistence(mut self, store: Arc<dyn JobStore>) -> Self {
        self.persistence = Some(store);
        self
    }

    /// Best-effort write-through to the backing store. A persistence failure is
    /// logged but never breaks the in-memory queue (availability over a single
    /// lost durability write); the next transition re-persists the latest state.
    async fn persist(&self, job: &Job) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.persist_job(job).await {
                tracing::warn!(job = %job.id, error = %e, "failed to persist job state");
            }
        }
    }

    /// Rebuild the in-memory index from the backing store. Jobs left `Running`
    /// by a crashed worker are reset to `Pending` so they are re-claimed —
    /// guaranteeing no outstanding work is lost across a restart. Returns the
    /// number of outstanding (pending/running) jobs recovered.
    pub async fn restore(&self) -> Result<usize> {
        let Some(store) = &self.persistence else {
            return Ok(0);
        };
        let jobs = store.load_jobs().await?;
        let mut recovered = 0usize;
        let mut requeued: Vec<Job> = Vec::new();
        {
            let mut state = self.state.write().await;
            state.jobs.clear();
            state.order.clear();
            for mut job in jobs {
                if job.status == JobStatus::Running {
                    // The worker that owned this job died; make it claimable again.
                    job.status = JobStatus::Pending;
                    job.ready_at = None;
                    job.updated_at = Utc::now();
                    requeued.push(job.clone());
                }
                if matches!(job.status, JobStatus::Pending | JobStatus::Running) {
                    recovered += 1;
                }
                state.order.push(job.id);
                state.jobs.insert(job.id, job);
            }
        }
        // Persist the Running -> Pending reset outside the lock.
        for job in &requeued {
            self.persist(job).await;
        }
        Ok(recovered)
    }

    /// Enqueue a new job, returning its id.
    pub async fn enqueue(&self, kind: impl Into<String>, payload: impl Into<String>, max_attempts: u32) -> Uuid {
        let job = Job::new(kind.into(), payload.into(), max_attempts);
        let id = job.id;
        {
            let mut state = self.state.write().await;
            state.order.push(id);
            state.jobs.insert(id, job.clone());
        }
        self.persist(&job).await;
        id
    }

    /// Claim the next ready pending job (FIFO), marking it `Running`.
    pub async fn claim_next(&self) -> Option<Job> {
        let now = Utc::now();
        let claimed = {
            let mut state = self.state.write().await;
            let ids: Vec<Uuid> = state.order.clone();
            let mut found = None;
            for id in ids {
                if let Some(job) = state.jobs.get(&id) {
                    if job.is_claimable(now) {
                        let job = state.jobs.get_mut(&id).unwrap();
                        job.status = JobStatus::Running;
                        job.attempts += 1;
                        job.ready_at = None;
                        job.updated_at = now;
                        found = Some(job.clone());
                        break;
                    }
                }
            }
            found
        };
        if let Some(ref job) = claimed {
            self.persist(job).await;
        }
        claimed
    }

    /// Mark a running job complete.
    pub async fn complete(&self, id: Uuid) {
        let updated = {
            let mut state = self.state.write().await;
            state.jobs.get_mut(&id).map(|job| {
                job.status = JobStatus::Completed;
                job.progress = 1.0;
                job.updated_at = Utc::now();
                job.clone()
            })
        };
        if let Some(ref job) = updated {
            self.persist(job).await;
        }
    }

    /// Record a failure. Re-queues with exponential backoff while attempts
    /// remain; otherwise marks the job `Failed`. Returns `true` if it will retry.
    pub async fn fail(&self, id: Uuid, error: impl Into<String>) -> bool {
        let (base, max) = (self.base_backoff_ms, self.max_backoff_ms);
        let (will_retry, updated) = {
            let mut state = self.state.write().await;
            if let Some(job) = state.jobs.get_mut(&id) {
                job.last_error = Some(error.into());
                job.updated_at = Utc::now();
                if job.attempts < job.max_attempts {
                    let shift = job.attempts.saturating_sub(1).min(20);
                    let backoff = base.saturating_mul(1u64 << shift).min(max);
                    job.ready_at =
                        Some(Utc::now() + chrono::Duration::milliseconds(backoff as i64));
                    job.status = JobStatus::Pending;
                    (true, Some(job.clone()))
                } else {
                    job.status = JobStatus::Failed;
                    (false, Some(job.clone()))
                }
            } else {
                (false, None)
            }
        };
        if let Some(ref job) = updated {
            self.persist(job).await;
        }
        will_retry
    }

    /// Cancel a pending or running job.
    pub async fn cancel(&self, id: Uuid) -> bool {
        let updated = {
            let mut state = self.state.write().await;
            match state.jobs.get_mut(&id) {
                Some(job) if matches!(job.status, JobStatus::Pending | JobStatus::Running) => {
                    job.status = JobStatus::Cancelled;
                    job.updated_at = Utc::now();
                    Some(job.clone())
                }
                _ => None,
            }
        };
        match updated {
            Some(ref job) => {
                self.persist(job).await;
                true
            }
            None => false,
        }
    }

    /// Update a running job's progress (clamped to `[0, 1]`).
    pub async fn set_progress(&self, id: Uuid, progress: f32) {
        let updated = {
            let mut state = self.state.write().await;
            state.jobs.get_mut(&id).map(|job| {
                job.progress = progress.clamp(0.0, 1.0);
                job.updated_at = Utc::now();
                job.clone()
            })
        };
        if let Some(ref job) = updated {
            self.persist(job).await;
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

    /// In-memory mock of a durable store to exercise write-through + restore
    /// without a real database.
    #[derive(Default)]
    struct MockJobStore {
        jobs: RwLock<HashMap<Uuid, Job>>,
    }
    #[async_trait]
    impl JobStore for MockJobStore {
        async fn persist_job(&self, job: &Job) -> Result<()> {
            self.jobs.write().await.insert(job.id, job.clone());
            Ok(())
        }
        async fn load_jobs(&self) -> Result<Vec<Job>> {
            Ok(self.jobs.read().await.values().cloned().collect())
        }
    }

    #[tokio::test]
    async fn write_through_persists_every_transition() {
        let store = Arc::new(MockJobStore::default());
        let q = JobQueue::with_backoff(0, 0).with_persistence(store.clone());
        let id = q.enqueue("test", "{}", 3).await;
        assert_eq!(store.jobs.read().await[&id].status, JobStatus::Pending);
        q.claim_next().await.unwrap();
        assert_eq!(store.jobs.read().await[&id].status, JobStatus::Running);
        q.complete(id).await;
        assert_eq!(store.jobs.read().await[&id].status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn restore_recovers_running_jobs_after_crash() {
        let store = Arc::new(MockJobStore::default());
        // Simulate a process that enqueued + claimed a job, then died mid-run.
        let id = {
            let q = JobQueue::with_backoff(0, 0).with_persistence(store.clone());
            let id = q.enqueue("paper_generation", "{}", 3).await;
            q.claim_next().await.unwrap(); // now Running, persisted as Running
            id
        };
        assert_eq!(store.jobs.read().await[&id].status, JobStatus::Running);

        // New process restores from the store: the orphaned Running job must
        // become claimable again so no work is lost.
        let q2 = JobQueue::with_backoff(0, 0).with_persistence(store.clone());
        let recovered = q2.restore().await.unwrap();
        assert_eq!(recovered, 1);
        assert_eq!(q2.get(id).await.unwrap().status, JobStatus::Pending);
        let reclaimed = q2.claim_next().await.unwrap();
        assert_eq!(reclaimed.id, id);
        assert_eq!(reclaimed.attempts, 2); // first attempt + this one
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
