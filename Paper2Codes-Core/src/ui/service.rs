// Integration service layer connecting UI to backend modules
// Implements event-driven architecture with async task processing

use crate::config::Config;
use crate::coordinator::Coordinator;
use crate::document::DocumentProcessor;
use crate::error::{Paper2CodesError, Result};
use crate::types::{Module, Paper};
use crate::ui::events::Event;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::error;

/// Service that bridges UI and backend processing
pub struct UIService {
    coordinator: Option<Arc<tokio::sync::Mutex<Coordinator>>>,
    document_processor: DocumentProcessor,
    event_tx: mpsc::UnboundedSender<Event>,
    processing_handle: Option<JoinHandle<Result<()>>>,
    pub config: Config,
}

impl UIService {
    pub fn new(event_tx: mpsc::UnboundedSender<Event>, config: Config) -> Self {
        Self {
            coordinator: None,
            document_processor: DocumentProcessor::new(),
            event_tx,
            processing_handle: None,
            config,
        }
    }

    /// Initialize the coordinator (gracefully handles missing API keys)
    pub async fn initialize(&mut self) -> Result<()> {
        if self.coordinator.is_none() {
            match Coordinator::new(self.config.clone()).await {
                Ok(coordinator) => {
                    self.coordinator = Some(Arc::new(tokio::sync::Mutex::new(coordinator)));
                    self.emit_event(Event::LogMessage(
                        "[INFO] Coordinator initialized successfully".to_string(),
                    ));
                }
                Err(e) => {
                    // Check if error is due to missing API keys
                    let error_msg = format!("{}", e);
                    if error_msg.contains("API key") || error_msg.contains("Missing required field")
                    {
                        self.emit_event(Event::LogMessage(
                            "[WARN] Coordinator not initialized: API keys missing. Please configure in Settings (press 's')".to_string(),
                        ));
                        // Don't fail - allow TUI to start without coordinator
                    } else {
                        // Other errors should still be reported
                        self.emit_event(Event::LogMessage(format!(
                            "[ERROR] Failed to initialize coordinator: {}",
                            e
                        )));
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Clear coordinator (to force reinitialization with new config)
    pub fn clear_coordinator(&mut self) {
        self.coordinator = None;
    }

    /// Update config and reinitialize the coordinator (called after API keys are updated)
    pub async fn update_config_and_reinitialize(&mut self, new_config: Config) -> Result<()> {
        self.config = new_config;
        // Clear existing coordinator
        self.coordinator = None;
        // Try to initialize again with new config
        self.initialize().await
    }

    /// Load and parse a paper from file path
    pub async fn load_paper(&mut self, paper_path: PathBuf) -> Result<Paper> {
        self.emit_event(Event::LogMessage(format!(
            "[INFO] Loading paper from: {}",
            paper_path.display()
        )));
        self.emit_event(Event::StatusUpdate(crate::ui::app::AppStatus::Processing));

        let paper = if paper_path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            self.document_processor.parse_pdf(&paper_path).await?
        } else {
            let content = std::fs::read_to_string(&paper_path)?;
            self.document_processor.parse_text(&content, None).await?
        };

        // Segment the paper
        let mut paper_clone = paper.clone();
        self.document_processor.segment_paper(&mut paper_clone)?;

        self.emit_event(Event::LogMessage(format!(
            "[INFO] Paper loaded: {} ({} segments)",
            paper.title,
            paper.segments.len()
        )));

        Ok(paper_clone)
    }

    /// Start processing a paper in the background
    pub async fn start_processing(&mut self, paper: Paper, output_dir: PathBuf) -> Result<()> {
        // Cancel any existing processing
        if let Some(handle) = self.processing_handle.take() {
            handle.abort();
        }

        // Ensure coordinator is initialized
        if self.coordinator.is_none() {
            self.initialize().await?;
        }

        let coordinator = self
            .coordinator
            .as_ref()
            .ok_or_else(|| {
                Paper2CodesError::Coordinator("Coordinator not initialized".to_string())
            })?
            .clone();

        let event_tx = self.event_tx.clone();

        // Spawn background task for processing
        let handle = tokio::spawn(async move {
            Self::process_paper_background(coordinator, paper, output_dir, event_tx).await
        });

        self.processing_handle = Some(handle);
        self.emit_event(Event::StatusUpdate(crate::ui::app::AppStatus::Processing));
        self.emit_event(Event::LogMessage(
            "[INFO] Started processing paper in background".to_string(),
        ));

        Ok(())
    }

    /// Background task that processes the paper and emits events
    async fn process_paper_background(
        coordinator: Arc<tokio::sync::Mutex<Coordinator>>,
        paper: Paper,
        output_dir: PathBuf,
        event_tx: mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let emit = |event: Event| {
            let _ = event_tx.send(event);
        };

        emit(Event::LogMessage(
            "[INFO] Starting paper processing".to_string(),
        ));
        emit(Event::StatusUpdate(crate::ui::app::AppStatus::Planning));
        emit(Event::ProgressUpdate(0.05));

        // Save paper to storage if available
        {
            let coord = coordinator.lock().await;
            if let Some(ref storage) = coord.storage_manager() {
                if storage.is_connected().await {
                    if let Err(e) = storage.save_paper(&paper).await {
                        emit(Event::LogMessage(format!(
                            "[WARN] Failed to save paper to storage: {}",
                            e
                        )));
                    } else {
                        emit(Event::LogMessage(
                            "[INFO] Paper saved to SurrealDB".to_string(),
                        ));
                    }

                    // Save paper segments
                    for segment in &paper.segments {
                        if let Err(e) = storage.save_segment(segment).await {
                            emit(Event::LogMessage(format!(
                                "[WARN] Failed to save segment {} to storage: {}",
                                segment.id, e
                            )));
                        }
                    }
                }
            }
        }

        // Update coordinator's repository output path
        {
            let mut coord = coordinator.lock().await;
            coord.set_output_path(output_dir);
        }

        // Process paper
        let result = {
            let mut coord = coordinator.lock().await;
            coord.process_paper(paper.clone()).await
        };

        match result {
            Ok(repository) => {
                emit(Event::LogMessage(
                    "[SUCCESS] Paper processing completed".to_string(),
                ));
                emit(Event::StatusUpdate(crate::ui::app::AppStatus::Completed));
                emit(Event::ProgressUpdate(1.0));

                // Save repository and modules to storage if available
                {
                    let coord = coordinator.lock().await;
                    if let Some(ref storage) = coord.storage_manager() {
                        if storage.is_connected().await {
                            // Save repository
                            if let Err(e) = storage.save_repository(&repository).await {
                                emit(Event::LogMessage(format!(
                                    "[WARN] Failed to save repository to storage: {}",
                                    e
                                )));
                            } else {
                                emit(Event::LogMessage(
                                    "[INFO] Repository saved to SurrealDB".to_string(),
                                ));
                            }

                            // Save modules
                            for module in &repository.modules {
                                let mut module_to_save = module.clone();
                                if module_to_save.repository_id.is_none() {
                                    module_to_save.repository_id =
                                        Some(repository.id.clone());
                                }
                                module_to_save.updated_at = Utc::now();

                                if let Err(e) = storage.save_module(&module_to_save).await {
                                    emit(Event::LogMessage(format!(
                                        "[WARN] Failed to save module {} to storage: {}",
                                        module.id, e
                                    )));
                                }
                            }
                        }
                    }
                }

                // Update UI with repository modules
                for module in &repository.modules {
                    let ui_module = Module {
                        id: module.id.clone(),
                        name: module
                            .file_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        description: format!("Module in {}", module.file_path.display()),
                        module_type: match module.language {
                            crate::types::ProgrammingLanguage::Python => {
                                crate::types::ModuleType::Script
                            }
                            crate::types::ProgrammingLanguage::Rust => {
                                crate::types::ModuleType::Class
                            }
                            _ => crate::types::ModuleType::Documentation,
                        },
                        dependencies: module.dependencies.clone(),
                        language: module.language.clone(),
                        status: crate::types::ModuleStatus::Completed,
                    };
                    emit(Event::ModuleUpdate(ui_module));
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("[ERROR] Processing failed: {}", e);
                error!("{}", error_msg);
                emit(Event::LogMessage(error_msg.clone()));
                emit(Event::StatusUpdate(crate::ui::app::AppStatus::Error(
                    error_msg,
                )));
                Err(e)
            }
        }
    }

    /// Get current coordinator state and update UI
    pub async fn sync_state(&self) -> Result<()> {
        if let Some(coordinator) = &self.coordinator {
            let coord = coordinator.lock().await;
            let state = coord.get_state();

            // Update tasks
            for task in state.get_all_tasks() {
                self.emit_event(Event::TaskUpdate(task));
            }

            // Update modules from repository
            for module in &state.repository.modules {
                let ui_module = Module {
                    id: module.id.clone(),
                    name: module
                        .file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    description: format!("Module in {}", module.file_path.display()),
                    module_type: match module.language {
                        crate::types::ProgrammingLanguage::Python => {
                            crate::types::ModuleType::Script
                        }
                        crate::types::ProgrammingLanguage::Rust => crate::types::ModuleType::Class,
                        _ => crate::types::ModuleType::Documentation,
                    },
                    dependencies: module.dependencies.clone(),
                    language: module.language.clone(),
                    status: crate::types::ModuleStatus::Completed,
                };
                self.emit_event(Event::ModuleUpdate(ui_module));
            }

            // Calculate progress
            let total_tasks = state.task_queue.total_count();
            let completed_tasks = state.task_queue.get_completed().len();
            let progress = if total_tasks > 0 {
                completed_tasks as f32 / total_tasks as f32
            } else {
                0.0
            };
            self.emit_event(Event::ProgressUpdate(progress));

            // Update status based on coordinator state
            let all_tasks = state.task_queue.get_all_tasks();
            let status = if all_tasks.is_empty() {
                if state
                    .feedback_history
                    .last()
                    .map(|r| r.passed)
                    .unwrap_or(false)
                {
                    crate::ui::app::AppStatus::Completed
                } else {
                    crate::ui::app::AppStatus::Error("Verification failed".to_string())
                }
            } else {
                // Determine current phase based on task types
                let current_task = all_tasks.first();
                match current_task {
                    Some(task) => match task.task_type {
                        crate::types::TaskType::Planning => crate::ui::app::AppStatus::Planning,
                        crate::types::TaskType::Analysis { .. } => {
                            crate::ui::app::AppStatus::Analyzing
                        }
                        crate::types::TaskType::Coding { .. } => crate::ui::app::AppStatus::Coding,
                        crate::types::TaskType::Verification { .. } => {
                            crate::ui::app::AppStatus::Verifying
                        }
                        _ => crate::ui::app::AppStatus::Processing,
                    },
                    None => crate::ui::app::AppStatus::Idle,
                }
            };
            self.emit_event(Event::StatusUpdate(status));
        }

        Ok(())
    }

    /// Get code content for a specific module
    pub async fn get_module_code(&self, module_id: &str) -> Result<Option<String>> {
        if let Some(coordinator) = &self.coordinator {
            let coord = coordinator.lock().await;
            let state = coord.get_state();

            if let Some(module) = state.repository.get_module(module_id) {
                return Ok(Some(module.content.clone()));
            }
        }
        Ok(None)
    }

    /// Stop current processing
    pub fn stop_processing(&mut self) {
        if let Some(handle) = self.processing_handle.take() {
            handle.abort();
            self.emit_event(Event::LogMessage(
                "[INFO] Processing stopped by user".to_string(),
            ));
            self.emit_event(Event::StatusUpdate(crate::ui::app::AppStatus::Idle));
        }
    }

    /// Emit an event to the UI
    fn emit_event(&self, event: Event) {
        let _ = self.event_tx.send(event);
    }

    /// Check if processing is active
    pub fn is_processing(&self) -> bool {
        self.processing_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}
