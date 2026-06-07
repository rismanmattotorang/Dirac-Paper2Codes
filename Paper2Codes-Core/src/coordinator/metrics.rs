//! Performance monitoring and metrics collection for Coordinator

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Performance metrics for coordinator operations
#[derive(Debug, Clone)]
pub struct CoordinatorMetrics {
    /// Total iterations executed
    pub total_iterations: u32,
    /// Total tasks completed
    pub total_tasks_completed: usize,
    /// Total tasks failed
    pub total_tasks_failed: usize,
    /// Average task duration in milliseconds
    pub avg_task_duration_ms: f64,
    /// Total LLM API calls made
    pub total_llm_calls: u64,
    /// Total tokens used (if tracked)
    pub total_tokens_used: Option<u64>,
    /// Average tokens per call
    pub avg_tokens_per_call: Option<f64>,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Current concurrency limit
    pub current_concurrency_limit: usize,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Per-agent metrics
    pub agent_metrics: HashMap<String, AgentMetrics>,
    /// Per-iteration metrics
    pub iteration_metrics: Vec<IterationMetrics>,
}

#[derive(Debug, Clone)]
pub struct AgentMetrics {
    pub calls: u64,
    pub total_duration_ms: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_duration_ms: f64,
    pub tokens_used: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct IterationMetrics {
    pub iteration: u32,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub duration_ms: u64,
    pub verification_passed: bool,
    pub issues_count: usize,
    pub converged: bool,
}

/// Metrics collector for coordinator
pub struct MetricsCollector {
    metrics: Arc<RwLock<CoordinatorMetrics>>,
    start_time: Instant,
    task_start_times: Arc<RwLock<HashMap<Uuid, Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(CoordinatorMetrics {
                total_iterations: 0,
                total_tasks_completed: 0,
                total_tasks_failed: 0,
                avg_task_duration_ms: 0.0,
                total_llm_calls: 0,
                total_tokens_used: None,
                avg_tokens_per_call: None,
                cache_hit_rate: 0.0,
                current_concurrency_limit: 0,
                success_rate: 0.0,
                total_execution_time: Duration::ZERO,
                agent_metrics: HashMap::new(),
                iteration_metrics: Vec::new(),
            })),
            start_time: Instant::now(),
            task_start_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_task_start(&self, task_id: Uuid) {
        let mut times = self.task_start_times.write().await;
        times.insert(task_id, Instant::now());
    }

    pub async fn record_task_completion(
        &self,
        task_id: Uuid,
        agent_type: &str,
        success: bool,
        duration_ms: u64,
        tokens_used: Option<u32>,
    ) {
        let mut metrics = self.metrics.write().await;
        let mut times = self.task_start_times.write().await;
        times.remove(&task_id);

        if success {
            metrics.total_tasks_completed += 1;
        } else {
            metrics.total_tasks_failed += 1;
        }

        // Update agent-specific metrics
        let agent_metric = metrics
            .agent_metrics
            .entry(agent_type.to_string())
            .or_insert_with(|| AgentMetrics {
                calls: 0,
                total_duration_ms: 0,
                successes: 0,
                failures: 0,
                avg_duration_ms: 0.0,
                tokens_used: None,
            });

        agent_metric.calls += 1;
        agent_metric.total_duration_ms += duration_ms;
        if success {
            agent_metric.successes += 1;
        } else {
            agent_metric.failures += 1;
        }
        agent_metric.avg_duration_ms =
            agent_metric.total_duration_ms as f64 / agent_metric.calls as f64;

        if let Some(tokens) = tokens_used {
            agent_metric.tokens_used = Some(agent_metric.tokens_used.unwrap_or(0) + tokens as u64);
        }

        // Update overall averages
        let total_tasks = metrics.total_tasks_completed + metrics.total_tasks_failed;
        if total_tasks > 0 {
            let total_duration = metrics
                .agent_metrics
                .values()
                .map(|m| m.total_duration_ms)
                .sum::<u64>();
            metrics.avg_task_duration_ms = total_duration as f64 / total_tasks as f64;
            metrics.success_rate = metrics.total_tasks_completed as f64 / total_tasks as f64;
        }
    }

    pub async fn record_llm_call(&self, tokens_used: Option<u32>, cache_hit: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_llm_calls += 1;

        if let Some(tokens) = tokens_used {
            metrics.total_tokens_used =
                Some(metrics.total_tokens_used.unwrap_or(0) + tokens as u64);
            if metrics.total_llm_calls > 0 {
                metrics.avg_tokens_per_call = Some(
                    metrics.total_tokens_used.unwrap() as f64 / metrics.total_llm_calls as f64,
                );
            }
        }

        // Update cache hit rate (simple moving average)
        let hit_weight = 0.1; // Weight for new observation
        if cache_hit {
            metrics.cache_hit_rate = metrics.cache_hit_rate * (1.0 - hit_weight) + 1.0 * hit_weight;
        } else {
            metrics.cache_hit_rate = metrics.cache_hit_rate * (1.0 - hit_weight) + 0.0 * hit_weight;
        }
    }

    pub async fn record_iteration(
        &self,
        iteration: u32,
        tasks_completed: usize,
        tasks_failed: usize,
        duration_ms: u64,
        verification_passed: bool,
        issues_count: usize,
        converged: bool,
    ) {
        let mut metrics = self.metrics.write().await;
        metrics.total_iterations = iteration + 1;
        metrics.total_execution_time = self.start_time.elapsed();

        metrics.iteration_metrics.push(IterationMetrics {
            iteration,
            tasks_completed,
            tasks_failed,
            duration_ms,
            verification_passed,
            issues_count,
            converged,
        });
    }

    pub async fn update_concurrency_limit(&self, limit: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.current_concurrency_limit = limit;
    }

    pub async fn get_metrics(&self) -> CoordinatorMetrics {
        let mut metrics = self.metrics.write().await;
        metrics.total_execution_time = self.start_time.elapsed();
        metrics.clone()
    }

    pub fn metrics(&self) -> Arc<RwLock<CoordinatorMetrics>> {
        Arc::clone(&self.metrics)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
