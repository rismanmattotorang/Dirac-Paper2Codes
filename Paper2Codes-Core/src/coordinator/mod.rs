mod metrics;

use crate::agents::{
    base::AgentResponse, Agent, AnalysisAgent, CodingAgent, PlanningAgent, VerificationAgent,
};
use crate::config::Config;
use crate::error::{Paper2CodesError, Result};
use crate::llm::LLMRouter;
use crate::performance::concurrency::{AdaptiveConcurrency, ConcurrencyConfig};
use crate::retrieval::{CPREngine, DefaultCPREngine};
use crate::storage::StorageManager;
use crate::types::{ImplementationPlan, Paper, Repository, Task, TaskType, VerificationReport};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use chrono::Utc;

pub use metrics::{AgentMetrics, CoordinatorMetrics, IterationMetrics, MetricsCollector};

pub struct Coordinator {
    config: Config,
    planning_agent: PlanningAgent,
    analysis_agent: AnalysisAgent,
    coding_agent: CodingAgent,
    verification_agent: VerificationAgent,
    llm_router: Arc<LLMRouter>,
    cpr_engine: Arc<dyn CPREngine>,
    storage_manager: Option<StorageManager>,
    state: CoordinatorState,
    concurrency_controller: Arc<AdaptiveConcurrency>,
    metrics_collector: Arc<MetricsCollector>,
}

pub struct CoordinatorState {
    pub paper: Option<Arc<Paper>>,
    pub plan: Option<Arc<ImplementationPlan>>,
    pub repository: Arc<Repository>,
    pub task_queue: TaskQueue,
    pub iteration: u32,
    pub feedback_history: Vec<VerificationReport>,
}

impl CoordinatorState {
    pub fn get_tasks(&self) -> Vec<Task> {
        self.task_queue.tasks.iter().cloned().collect()
    }

    pub fn get_completed_tasks(&self) -> Vec<Task> {
        self.task_queue.completed.values().cloned().collect()
    }

    pub fn get_failed_tasks(&self) -> Vec<Task> {
        self.task_queue.failed.values().cloned().collect()
    }

    pub fn get_all_tasks(&self) -> Vec<Task> {
        let mut all = self.get_tasks();
        all.extend(self.get_completed_tasks());
        all.extend(self.get_failed_tasks());
        all
    }
}

pub struct TaskQueue {
    tasks: VecDeque<Task>,
    completed: HashMap<Uuid, Task>,
    failed: HashMap<Uuid, Task>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            completed: HashMap::new(),
            failed: HashMap::new(),
        }
    }

    pub fn from_plan(plan: &ImplementationPlan) -> Self {
        let mut queue = Self::new();

        // Create analysis tasks for each module
        for module in &plan.modules {
            let task = Task::new(
                TaskType::Analysis {
                    module_id: module.id.clone(),
                },
                format!("Analyze module: {}", module.name),
            );
            queue.add_task(task);
        }

        // Create coding tasks (will be added after analysis)
        // Dependencies will be handled by coordinator

        queue
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    pub fn get_ready_tasks(&self) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|task| {
                // Check if all dependencies are completed
                task.dependencies
                    .iter()
                    .all(|dep_id| self.completed.contains_key(dep_id))
            })
            .cloned()
            .collect()
    }

    pub fn mark_completed(&mut self, task_id: &Uuid) {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == *task_id) {
            if let Some(task) = self.tasks.remove(pos) {
                self.completed.insert(*task_id, task);
            }
        }
    }

    pub fn mark_failed(&mut self, task_id: &Uuid, error: String) {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == *task_id) {
            if let Some(mut task) = self.tasks.remove(pos) {
                task.mark_failed(error);
                self.failed.insert(*task_id, task);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn has_ready_tasks(&self) -> bool {
        !self.get_ready_tasks().is_empty()
    }

    pub fn get_all_tasks(&self) -> Vec<Task> {
        self.tasks.iter().cloned().collect()
    }

    pub fn get_completed(&self) -> &HashMap<Uuid, Task> {
        &self.completed
    }

    pub fn get_failed(&self) -> &HashMap<Uuid, Task> {
        &self.failed
    }

    pub fn total_count(&self) -> usize {
        self.tasks.len() + self.completed.len() + self.failed.len()
    }
}

impl Coordinator {
    pub async fn new(config: Config) -> Result<Self> {
        let llm_router = Arc::new(LLMRouter::new(config.clone())?);

        // Build the CPR engine, wiring optional Phase 3 augmentation when the
        // required services are available. These are capability-gated feature
        // flags: dense retrieval + HyDE activate only with an embedding key, and
        // HyDE / reranking activate only with an LLM client. Without them,
        // retrieval falls back to the Phase 1/2 BM25 + RRF hybrid pipeline.
        let mut engine = DefaultCPREngine::new();
        if let Some(openai_key) = config.get_api_key("openai") {
            match crate::retrieval::EmbeddingService::new_openai(openai_key) {
                Ok(service) => {
                    engine = engine.with_embedding_service(Arc::new(service));
                    tracing::info!("CPR: dense embedding retrieval enabled (OpenAI)");
                }
                Err(e) => tracing::warn!("CPR: embedding service unavailable: {}", e),
            }
        }
        if let Some(client) = llm_router.default_client() {
            engine = engine
                .with_query_expander(Arc::new(crate::retrieval::LlmQueryExpander::new(
                    client.clone(),
                )))
                .with_reranker(Arc::new(crate::retrieval::LlmReranker::new(client)));
            tracing::info!("CPR: HyDE query expansion + LLM reranking enabled");
        }
        let cpr_engine: Arc<dyn CPREngine> = Arc::new(engine);

        // Initialize storage manager if storage is enabled
        let mut storage_manager = None;
        if config.storage.enabled {
            let manager = StorageManager::new();
            if let Err(e) = manager.connect(config.storage.clone()).await {
                tracing::warn!(
                    "Failed to connect to storage: {}. Continuing without persistence.",
                    e
                );
            } else {
                tracing::info!("Connected to SurrealDB storage");
                storage_manager = Some(manager);
            }
        }

        // Initialize adaptive concurrency controller
        let concurrency_config = ConcurrencyConfig {
            initial_limit: config.agents.parallel_tasks,
            min_limit: 1,
            max_limit: config.agents.parallel_tasks * 2,
            step_size: 1,
            success_threshold: 0.8,
            window_size: 10,
        };
        let concurrency_controller = Arc::new(AdaptiveConcurrency::new(concurrency_config));

        // Initialize metrics collector
        let metrics_collector = Arc::new(MetricsCollector::new());

        Ok(Self {
            config: config.clone(),
            planning_agent: PlanningAgent::new(),
            analysis_agent: AnalysisAgent::new(),
            coding_agent: CodingAgent::new(),
            verification_agent: VerificationAgent::with_config(&config),
            llm_router,
            cpr_engine,
            storage_manager,
            concurrency_controller,
            metrics_collector,
            state: CoordinatorState {
                paper: None,
                plan: None,
                repository: Arc::new(Repository::new(PathBuf::from("./output"))),
                task_queue: TaskQueue::new(),
                iteration: 0,
                feedback_history: Vec::new(),
            },
        })
    }

    /// Get metrics collector reference
    pub fn metrics(&self) -> Arc<MetricsCollector> {
        Arc::clone(&self.metrics_collector)
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> CoordinatorMetrics {
        self.metrics_collector.get_metrics().await
    }

    /// Get storage manager reference
    pub fn storage_manager(&self) -> Option<&StorageManager> {
        self.storage_manager.as_ref()
    }

    /// Get mutable storage manager reference
    pub fn storage_manager_mut(&mut self) -> Option<&mut StorageManager> {
        self.storage_manager.as_mut()
    }

    pub async fn process_paper(&mut self, paper: Paper) -> Result<Repository> {
        // Set paper in state
        self.state.paper = Some(Arc::new(paper.clone()));

        // Save paper to storage if available
        if let Some(ref mut storage) = self.storage_manager {
            if storage.is_connected().await {
                if let Err(e) = self.save_paper_to_storage(&paper).await {
                    tracing::warn!("Failed to save paper to storage: {}", e);
                } else {
                    tracing::info!("Saved paper to storage: {}", paper.id);
                }
            }
        }

        // Initialize retrieval (CPR) engine with paper context
        if let Err(e) = self.cpr_engine.initialize(&paper).await {
            tracing::warn!("Failed to initialize retrieval engine: {}", e);
        }

        // Generate implementation plan
        let plan = self.generate_plan(&paper).await?;
        self.state.plan = Some(Arc::new(plan.clone()));

        // Initialize task queue from plan
        self.state.task_queue = TaskQueue::from_plan(&plan);

        // Save initial tasks to storage
        if let Some(ref mut storage) = self.storage_manager {
            if storage.is_connected().await {
                for task in &self.state.task_queue.tasks {
                    if let Err(e) = self.save_task_to_storage(task).await {
                        tracing::warn!("Failed to save task to storage: {}", e);
                    }
                }
            }
        }

        // Add coding tasks after analysis tasks, wiring cross-module dependencies
        // from the plan's dependency graph so a module is generated only after the
        // modules it depends on (Phase 6 graph-native generation ordering).
        use crate::storage::graph::GraphAnalyzer;

        let module_ids: Vec<String> = plan.modules.iter().map(|m| m.id.clone()).collect();
        let order = GraphAnalyzer::topological_order(&module_ids, &plan.dependencies);
        let rank: std::collections::HashMap<String, usize> = order
            .iter()
            .enumerate()
            .map(|(i, m)| (m.clone(), i))
            .collect();

        // First pass: build coding tasks (each depending on its analysis task)
        // and map module id -> coding task id.
        let mut coding_tasks: Vec<(String, Task)> = Vec::new();
        let mut coding_id_by_module: std::collections::HashMap<String, Uuid> =
            std::collections::HashMap::new();

        for module in &plan.modules {
            let analysis_task_id = self
                .state
                .task_queue
                .tasks
                .iter()
                .find(|t| {
                    matches!(&t.task_type, TaskType::Analysis { module_id } if module_id == &module.id)
                })
                .map(|t| t.id);

            // GraphRAG-style hint: surface the module's transitive dependencies
            // (up to 2 hops) so the coder knows what it can build on.
            let deps = GraphAnalyzer::dependency_closure(&plan.dependencies, &module.id, 2);
            let description = if deps.is_empty() {
                format!("Implement module: {}", module.name)
            } else {
                format!(
                    "Implement module: {} (depends on: {})",
                    module.name,
                    deps.join(", ")
                )
            };

            let mut coding_task = Task::new(
                TaskType::Coding {
                    module_id: module.id.clone(),
                },
                description,
            );
            if let Some(analysis_id) = analysis_task_id {
                coding_task.dependencies.push(analysis_id);
            }
            coding_id_by_module.insert(module.id.clone(), coding_task.id);
            coding_tasks.push((module.id.clone(), coding_task));
        }

        // Second pass: add cross-module coding dependencies, but only "backward"
        // along the topological order. This guarantees the task-dependency graph
        // stays acyclic even if the module graph has cycles, so the readiness
        // check can never deadlock.
        for (module_id, task) in coding_tasks.iter_mut() {
            let my_rank = rank.get(module_id).copied().unwrap_or(usize::MAX);
            for dep in plan.dependencies.get_dependencies(module_id) {
                if let Some(dep_task_id) = coding_id_by_module.get(&dep) {
                    let dep_rank = rank.get(&dep).copied().unwrap_or(usize::MAX);
                    if dep_rank < my_rank && !task.dependencies.contains(dep_task_id) {
                        task.dependencies.push(*dep_task_id);
                    }
                }
            }
        }

        for (_module_id, task) in coding_tasks {
            self.state.task_queue.add_task(task);
        }

        // Main iteration loop
        let max_iterations = self.config.agents.max_iterations;
        for iteration in 0..max_iterations {
            self.state.iteration = iteration;

            // Execute iteration
            let result = self.execute_iteration().await?;

            if result.converged {
                break;
            }

            if self.state.task_queue.is_empty() && result.verification_passed {
                break;
            }
        }

        Ok((*self.state.repository).clone())
    }

    async fn generate_plan(&self, paper: &Paper) -> Result<ImplementationPlan> {
        let task = Task::new(
            TaskType::Planning,
            format!("Generate implementation plan for: {}", paper.title),
        );

        let context = self
            .create_agent_context(paper, None, &self.state.repository)
            .await?;

        let response = self.planning_agent.execute(&task, &context).await?;

        match response.result {
            crate::types::AgentResult::Plan(plan) => Ok(plan),
            _ => Err(Paper2CodesError::Agent(
                crate::error::AgentError::InvalidTaskType(
                    "Expected Plan result from planning agent".to_string(),
                ),
            )),
        }
    }

    pub async fn execute_iteration(&mut self) -> Result<IterationResult> {
        let iteration_start = Instant::now();
        let mut verification_passed = false;
        let mut tasks_completed_this_iteration = 0;
        let mut tasks_failed_this_iteration = 0;

        // Process ready tasks in parallel using adaptive concurrency control
        let ready_tasks = self.state.task_queue.get_ready_tasks();
        let paper = self
            .state
            .paper
            .as_ref()
            .ok_or_else(|| {
                Paper2CodesError::Coordinator("Paper not set in coordinator state".to_string())
            })?
            .clone();
        let plan = self.state.plan.as_ref().cloned();
        let repo_arc = self.state.repository.clone();

        // Use adaptive concurrency controller instead of fixed semaphore
        let concurrency_controller = Arc::clone(&self.concurrency_controller);
        let metrics_collector = Arc::clone(&self.metrics_collector);

        // Execute all ready tasks concurrently with adaptive concurrency control
        let mut task_handles: Vec<
            tokio::task::JoinHandle<Result<(Uuid, Result<AgentResponse>, String)>>,
        > = Vec::new();

        for task in ready_tasks {
            let task_id = task.id;
            let task_clone = task.clone();
            let paper_clone = paper.clone();
            let plan_clone = plan.clone();
            let repo_clone = repo_arc.clone();
            let cpr_engine_clone = self.cpr_engine.clone();
            let llm_router_clone = self.llm_router.clone();
            let concurrency_controller_clone = Arc::clone(&concurrency_controller);
            let metrics_collector_clone = Arc::clone(&metrics_collector);

            // Record task start
            metrics_collector_clone.record_task_start(task_id).await;

            // Clone agents
            let analysis_agent = self.analysis_agent.clone();
            let coding_agent = self.coding_agent.clone();
            let verification_agent = self.verification_agent.clone();

            // Determine agent type for metrics
            let agent_type = match task_clone.task_type {
                TaskType::Analysis { .. } => "analysis",
                TaskType::Coding { .. } => "coding",
                TaskType::Verification { .. } => "verification",
                TaskType::Fix { .. } => "fix",
                _ => "unknown",
            };

            let handle = tokio::spawn(async move {
                // Acquire adaptive concurrency permit
                let permit = match concurrency_controller_clone.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        return Ok((
                            task_id,
                            Err(Paper2CodesError::Coordinator(format!(
                                "Failed to acquire concurrency permit: {}",
                                e
                            ))),
                            agent_type.to_string(),
                        ));
                    }
                };

                let task_start = Instant::now();

                // Create agent context
                let context = crate::agents::base::AgentContext {
                    paper: paper_clone,
                    plan: plan_clone,
                    repository: repo_clone,
                    cpr_engine: cpr_engine_clone,
                    llm_router: llm_router_clone,
                };

                // Execute task based on type
                let result = match task_clone.task_type {
                    TaskType::Analysis { .. } => {
                        tracing::info!("Executing analysis task: {}", task_id);
                        analysis_agent.execute(&task_clone, &context).await
                    }
                    TaskType::Coding { .. } => {
                        tracing::info!("Executing coding task: {}", task_id);
                        coding_agent.execute(&task_clone, &context).await
                    }
                    TaskType::Verification { .. } => {
                        tracing::info!("Executing verification task: {}", task_id);
                        verification_agent.execute(&task_clone, &context).await
                    }
                    TaskType::Fix { .. } => {
                        tracing::info!("Executing fix task: {}", task_id);
                        coding_agent.execute(&task_clone, &context).await
                    }
                    _ => {
                        tracing::warn!("Unknown task type for task: {}", task_id);
                        Err(crate::error::Paper2CodesError::Agent(
                            crate::error::AgentError::InvalidTaskType(
                                "Unknown task type".to_string(),
                            ),
                        ))
                    }
                };

                let duration_ms = task_start.elapsed().as_millis() as u64;
                let success = result.is_ok();

                // Record result for adaptive concurrency
                concurrency_controller_clone.record_result(success).await;

                // Record metrics
                let tokens_used = result.as_ref().ok().and_then(|r| r.metadata.tokens_used);
                metrics_collector_clone
                    .record_task_completion(task_id, agent_type, success, duration_ms, tokens_used)
                    .await;

                drop(permit);

                Ok((task_id, result, agent_type.to_string()))
            });

            task_handles.push(handle);
        }

        // Collect results from all tasks
        let task_results = futures::future::join_all(task_handles).await;

        // Process results in order to maintain consistency
        for handle_result in task_results {
            match handle_result {
                Ok(Ok((task_id, result, agent_type))) => {
                    match result {
                        Ok(response) => {
                            tracing::info!("Task {} completed successfully", task_id);
                            tasks_completed_this_iteration += 1;
                            let mut should_continue = false;
                            let agent_result = response.result;

                            match agent_result {
                                crate::types::AgentResult::Analysis(_spec) => {
                                    // Store specification in task context for coding tasks
                                    tracing::debug!("Analysis completed for task {}", task_id);
                                }
                                crate::types::AgentResult::Code(mut code) => {
                                    // Add code to repository atomically
                                    tracing::info!("Code generated for module: {}", code.id);

                                    // Check storage connection before mutable borrow
                                    let should_save_to_storage =
                                        if let Some(ref storage) = self.storage_manager {
                                            storage.is_connected().await
                                        } else {
                                            false
                                        };

                                    if code.repository_id.is_none() {
                                        code.repository_id =
                                            Some(self.state.repository.id.clone());
                                    }
                                    code.updated_at = Utc::now();

                                    // Save module to storage before adding to repo
                                    if should_save_to_storage {
                                        if let Some(ref storage) = self.storage_manager {
                                            if let Err(e) = storage.save_module(&code).await {
                                                tracing::warn!(
                                                    "Failed to save module to storage: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }

                                    // Use Arc::make_mut for efficient cloning only when needed
                                    let repo = Arc::make_mut(&mut self.state.repository);
                                    repo.add_module(code.clone());

                                    // Update repository in storage (after releasing mutable borrow)
                                    if should_save_to_storage {
                                        // Clone repo for storage update (to avoid borrow conflict)
                                        let repo_clone = repo.clone();
                                        let storage_manager_ref = self.storage_manager.as_ref();

                                        if let Some(storage) = storage_manager_ref {
                                            // Try update first, fallback to save if update fails
                                            if storage.update_repository(&repo_clone).await.is_err()
                                            {
                                                if let Err(e) =
                                                    storage.save_repository(&repo_clone).await
                                                {
                                                    tracing::warn!(
                                                        "Failed to save repository to storage: {}",
                                                        e
                                                    );
                                                } else {
                                                    tracing::debug!("Saved repository to storage after update failed");
                                                }
                                            } else {
                                                tracing::debug!("Updated repository in storage");
                                            }
                                        }
                                    }
                                }
                                crate::types::AgentResult::Verification(report) => {
                                    verification_passed = report.passed;
                                    tracing::info!(
                                        "Verification completed with {} issues",
                                        report.issues.len()
                                    );
                                    let report_clone = report.clone();
                                    self.state.feedback_history.push(report_clone.clone());
                                    self.state.task_queue.mark_completed(&task_id);
                                    self.handle_verification_feedback(report_clone).await?;
                                    should_continue = true;
                                }
                                crate::types::AgentResult::Plan(_) => {
                                    tracing::debug!("Plan generated for task {}", task_id);
                                }
                            }

                            if !should_continue {
                                self.state.task_queue.mark_completed(&task_id);

                                // Update task status in storage
                                if let Some(ref mut storage) = self.storage_manager {
                                    if storage.is_connected().await {
                                        if let Some(task) =
                                            self.state.task_queue.completed.get(&task_id)
                                        {
                                            if let Err(e) = self.save_task_to_storage(task).await {
                                                tracing::warn!(
                                                    "Failed to update task in storage: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(task_id = %task_id, agent_type = %agent_type, error = %e, "Task failed");
                            tasks_failed_this_iteration += 1;
                            let error_msg = format!("{}", e);
                            self.state
                                .task_queue
                                .mark_failed(&task_id, error_msg.clone());

                            // Update task status in storage
                            if let Some(ref mut storage) = self.storage_manager {
                                if storage.is_connected().await {
                                    if let Some(task) = self.state.task_queue.failed.get(&task_id) {
                                        if let Err(storage_err) =
                                            self.save_task_to_storage(task).await
                                        {
                                            tracing::warn!(task_id = %task_id, error = %storage_err, "Failed to update failed task in storage");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "Task execution wrapper failed");
                    tasks_failed_this_iteration += 1;
                }
                Err(join_error) => {
                    tracing::error!(error = %join_error, "Task execution panicked");
                    tasks_failed_this_iteration += 1;
                    // Try to mark the task as failed if we can identify it
                    // Note: We don't have task_id here, so we can't update the queue
                }
            }
        }

        // Run verification if all coding tasks are done
        if self.state.task_queue.is_empty() {
            tracing::info!("All tasks completed, running final verification");
            let verification_task = Task::new(
                TaskType::Verification { module_id: None },
                "Verify entire repository".to_string(),
            );

            let repo_clone = (*self.state.repository).clone();
            let context = self.create_agent_context(&paper, plan, &repo_clone).await?;

            match self
                .verification_agent
                .execute(&verification_task, &context)
                .await
            {
                Ok(response) => {
                    if let crate::types::AgentResult::Verification(report) = response.result {
                        verification_passed = report.passed;
                        tracing::info!(
                            passed = report.passed,
                            issues_count = report.issues.len(),
                            "Final verification completed"
                        );
                        self.state.feedback_history.push(report.clone());
                        if let Err(e) = self.handle_verification_feedback(report).await {
                            tracing::warn!(error = %e, "Failed to handle verification feedback");
                        }
                    } else {
                        tracing::warn!("Verification agent returned unexpected result type");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Final verification failed");
                    // Don't fail the entire iteration, but mark verification as not passed
                    verification_passed = false;
                }
            }
        }

        // Check convergence
        let converged = self.has_converged();
        let issues_count = self
            .state
            .feedback_history
            .last()
            .map(|r| r.issues.len())
            .unwrap_or(0);

        // Record iteration metrics
        let iteration_duration_ms = iteration_start.elapsed().as_millis() as u64;
        self.metrics_collector
            .record_iteration(
                self.state.iteration,
                tasks_completed_this_iteration,
                tasks_failed_this_iteration,
                iteration_duration_ms,
                verification_passed,
                issues_count,
                converged,
            )
            .await;

        // Update concurrency limit in metrics
        let current_limit = self.concurrency_controller.current_limit().await;
        self.metrics_collector
            .update_concurrency_limit(current_limit)
            .await;

        Ok(IterationResult {
            verification_passed,
            converged,
            tasks_completed: self.state.task_queue.completed.len(),
        })
    }

    async fn handle_verification_feedback(&mut self, feedback: VerificationReport) -> Result<()> {
        // Phase 4 self-debugging: group actionable (critical/error) issues by
        // module and emit ONE structured, severity-prioritised repair brief per
        // module rather than a separate fix task per symptom. This gives the
        // coder the full picture of a module's problems in a single prompt.
        let actionable: Vec<crate::types::VerificationIssue> = feedback
            .issues
            .iter()
            .filter(|i| {
                i.severity == crate::types::IssueSeverity::Critical
                    || i.severity == crate::types::IssueSeverity::Error
            })
            .cloned()
            .collect();

        if actionable.is_empty() {
            return Ok(());
        }

        let (groups, _unlocated) = crate::agents::repair::group_by_module(&actionable);
        for (module, issues) in groups {
            let brief = crate::agents::repair::format_repair_feedback(&module, &issues);
            let fix_task = Task::new(
                TaskType::Fix {
                    module_id: module,
                    feedback: brief.clone(),
                },
                brief,
            );
            self.state.task_queue.add_task(fix_task);
        }

        Ok(())
    }

    /// Check convergence using multiple criteria based on state-of-the-art early stopping
    /// Uses adaptive convergence detection with multiple signals
    fn has_converged(&self) -> bool {
        // Need at least 2 iterations to check convergence
        if self.state.feedback_history.len() < 2 {
            return false;
        }

        // Need minimum iterations before considering convergence
        if self.state.iteration < 2 {
            return false;
        }

        let last = match self.state.feedback_history.last() {
            Some(v) => v,
            None => return false,
        };
        let _prev = match self
            .state
            .feedback_history
            .get(self.state.feedback_history.len() - 2)
        {
            Some(v) => v,
            None => return false,
        };

        // Convergence criteria (ordered by priority):

        // 1. Perfect convergence: All tests passing and no issues
        if last.passed && last.issues.is_empty() {
            tracing::info!("Converged: All tests passing with no issues");
            return true;
        }

        // 2. Plateau detection: No improvement in issue count for multiple iterations
        // This uses a more sophisticated approach than simple comparison
        if self.state.feedback_history.len() >= 3 {
            let issue_counts: Vec<usize> = self
                .state
                .feedback_history
                .iter()
                .rev()
                .take(3)
                .map(|r| r.issues.len())
                .collect();

            // Check if issue count has plateaued (no significant decrease)
            let avg_recent = issue_counts.iter().sum::<usize>() as f32 / issue_counts.len() as f32;
            let variance = issue_counts
                .iter()
                .map(|&x| (x as f32 - avg_recent).powi(2))
                .sum::<f32>()
                / issue_counts.len() as f32;

            // If variance is low and we're not improving, consider converged
            if variance < 1.0
                && issue_counts[0] >= issue_counts[1]
                && issue_counts[1] >= issue_counts[2]
            {
                tracing::info!(
                    "Converged: Plateau detected (no improvement in {} iterations)",
                    issue_counts.len()
                );
                return true;
            }
        }

        // 3. Maximum iterations reached
        if self.state.iteration >= self.config.agents.max_iterations {
            tracing::info!("Converged: Maximum iterations reached");
            return true;
        }

        // 4. Quality-based convergence: All critical issues resolved
        let critical_issues = last
            .issues
            .iter()
            .filter(|i| i.severity == crate::types::IssueSeverity::Critical)
            .count();
        let error_issues = last
            .issues
            .iter()
            .filter(|i| i.severity == crate::types::IssueSeverity::Error)
            .count();

        // Accept convergence if no critical issues and minimal errors
        if critical_issues == 0
            && error_issues <= 2
            && last.issues.len() <= 5
            && self.state.iteration >= 2
        {
            tracing::info!("Converged: All critical issues resolved, only minor issues remain");
            return true;
        }

        // 5. Task queue empty and verification passed
        if self.state.task_queue.is_empty() && last.passed {
            tracing::info!("Converged: All tasks completed and verification passed");
            return true;
        }

        // 6. Diminishing returns: Improvement rate is very low
        if self.state.feedback_history.len() >= 4 {
            let recent_improvements: Vec<f32> = self
                .state
                .feedback_history
                .iter()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .windows(2)
                .map(|w| {
                    let improvement = w[1].issues.len() as f32 - w[0].issues.len() as f32;
                    improvement
                })
                .collect();

            // If average improvement is very small (less than 0.5 issues per iteration)
            let avg_improvement =
                recent_improvements.iter().sum::<f32>() / recent_improvements.len() as f32;
            if avg_improvement.abs() < 0.5 && self.state.iteration >= 4 {
                tracing::info!(
                    "Converged: Diminishing returns detected (avg improvement: {:.2})",
                    avg_improvement
                );
                return true;
            }
        }

        false
    }

    async fn create_agent_context(
        &self,
        paper: &Paper,
        plan: Option<Arc<ImplementationPlan>>,
        repository: &Repository,
    ) -> Result<crate::agents::base::AgentContext> {
        Ok(crate::agents::base::AgentContext {
            paper: Arc::new(paper.clone()),
            plan,
            repository: Arc::new(repository.clone()),
            cpr_engine: self.cpr_engine.clone(),
            llm_router: self.llm_router.clone(),
        })
    }

    pub fn get_state(&self) -> &CoordinatorState {
        &self.state
    }

    pub fn set_output_path(&mut self, path: PathBuf) {
        self.state.repository = Arc::new(Repository::new(path));
    }

    // Storage persistence helpers
    async fn save_paper_to_storage(&self, paper: &Paper) -> Result<()> {
        if let Some(ref storage) = self.storage_manager {
            storage.save_paper(paper).await?;
        }
        Ok(())
    }

    async fn save_task_to_storage(&self, task: &Task) -> Result<()> {
        if let Some(ref storage) = self.storage_manager {
            storage.save_task(task).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn save_module_to_storage(
        &self,
        module: &crate::types::CodeModule,
        _repo_id: &str,
    ) -> Result<()> {
        if let Some(ref storage) = self.storage_manager {
            storage.save_module(module).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    async fn save_repository_to_storage(&self, repo: &Repository) -> Result<()> {
        if let Some(ref storage) = self.storage_manager {
            // Try update first, if it fails, try save
            if storage.update_repository(repo).await.is_err() {
                storage.save_repository(repo).await?;
            }
        }
        Ok(())
    }
}

pub struct IterationResult {
    pub verification_passed: bool,
    pub converged: bool,
    pub tasks_completed: usize,
}
