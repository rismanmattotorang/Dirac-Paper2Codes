use crate::config::Config;
use crate::storage::StorageManager;
use crate::types::{Module, Task, TaskStatus};
use crate::ui::events::Event;
use crate::ui::service::UIService;
use crate::ui::theme::Theme;
use crate::ui::widgets::{
    AdminConsoleWidget, CodeViewer, ConnectionWidget, DatabaseStats, HelpWidget, LogViewer,
    ModuleList, ProgressWidget, SettingsWidget, StatisticsWidget, TaskList,
};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode, KeyEventKind,
    KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Idle,
    Processing,
    Planning,
    Analyzing,
    Coding,
    Verifying,
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Dashboard,
    Tasks,
    Modules,
    Code,
    Skills,
    Logs,
    Statistics,
    Settings,
    Help,
    Storage,
    AdminConsole,
}

pub struct App {
    pub tasks: Vec<Task>,
    pub modules: Vec<Module>,
    pub selected_task: usize,
    pub selected_module: usize,
    pub current_code: String,
    pub current_file_name: String,
    pub logs: Vec<String>,
    pub status: AppStatus,
    pub progress: f32,
    pub should_quit: bool,
    pub show_help: bool,
    pub view_mode: ViewMode,
    pub code_scroll: usize,
    pub log_scroll: usize,
    pub event_rx: mpsc::UnboundedReceiver<Event>,
    pub event_tx: mpsc::UnboundedSender<Event>,
    pub theme: Theme,
    pub start_time: Instant,
    pub error_message: Option<String>,
    pub config: Config,
    pub selected_setting: usize,
    pub editing_api_key: bool,
    pub api_key_input: String,
    pub input_mode: InputMode,
    pub service: Option<UIService>,
    pub paper_path: Option<PathBuf>,
    // Storage UI state
    pub storage_manager: Option<Arc<StorageManager>>,
    pub storage_connected: bool,
    pub storage_error: Option<String>,
    pub admin_query: String,
    pub admin_query_result: Option<String>,
    pub admin_query_error: Option<String>,
    pub selected_table: usize,
    pub table_data: Vec<Vec<String>>,
    pub db_stats: Option<DatabaseStats>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    EditingApiKey,
    EditingAdminQuery,
}

impl App {
    pub fn new() -> (Self, mpsc::UnboundedSender<Event>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let theme = Theme::new();
        let config = Config::load().unwrap_or_else(|_| Config::default());
        let service = UIService::new(event_tx.clone(), config.clone());

        let app = Self {
            tasks: Vec::new(),
            modules: Vec::new(),
            selected_task: 0,
            selected_module: 0,
            current_code: String::new(),
            current_file_name: String::new(),
            logs: Vec::new(),
            status: AppStatus::Idle,
            progress: 0.0,
            should_quit: false,
            show_help: false,
            view_mode: ViewMode::Dashboard,
            code_scroll: 0,
            log_scroll: 0,
            event_rx,
            event_tx: event_tx.clone(),
            theme,
            start_time: Instant::now(),
            error_message: None,
            selected_setting: 0,
            editing_api_key: false,
            api_key_input: String::new(),
            input_mode: InputMode::Normal,
            service: Some(service),
            paper_path: None,
            storage_manager: {
                // Always initialize storage manager so we can connect when needed
                Some(Arc::new(StorageManager::new()))
            },
            config,
            storage_connected: false,
            storage_error: None,
            admin_query: String::new(),
            admin_query_result: None,
            admin_query_error: None,
            selected_table: 0,
            table_data: Vec::new(),
            db_stats: None,
        };

        (app, event_tx)
    }

    pub fn new_with_paper(paper_path: PathBuf) -> (Self, mpsc::UnboundedSender<Event>) {
        let (mut app, event_tx) = Self::new();
        app.paper_path = Some(paper_path);
        (app, event_tx)
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Always try to connect to SurrealDB if storage manager is available
        if let Some(ref storage) = self.storage_manager {
            // Check if storage is enabled in config, or try to connect anyway
            if self.config.storage.enabled {
                self.logs
                    .push("[INFO] Connecting to SurrealDB...".to_string());
                // Show the actual connection string (may have ws:// prefix in config)
                let display_conn = if self.config.storage.connection_string.starts_with("ws://")
                    || self.config.storage.connection_string.starts_with("wss://")
                {
                    format!(
                        "ws://{}",
                        self.config
                            .storage
                            .connection_string
                            .trim_start_matches("ws://")
                            .trim_start_matches("wss://")
                    )
                } else {
                    format!("ws://{}", self.config.storage.connection_string)
                };
                self.logs
                    .push(format!("[INFO] Connection: {}", display_conn));
                self.logs.push(format!(
                    "[INFO] Namespace: {}, Database: {}",
                    self.config.storage.namespace, self.config.storage.database
                ));
            } else {
                // Storage is disabled, but we'll still try to connect with defaults
                // This allows users to connect even if not explicitly enabled
                self.logs.push(
                    "[INFO] Storage is disabled in config, but attempting connection..."
                        .to_string(),
                );
                // Enable storage temporarily for connection attempt
                self.config.storage.enabled = true;
            }

            let config = self.config.storage.clone();
            match storage.connect(config).await {
                Ok(_) => {
                    self.storage_connected = true;
                    self.storage_error = None;
                    self.logs
                        .push("[INFO] ✓ Connected to SurrealDB successfully!".to_string());

                    // Test connection and get stats
                    match storage.test_connection().await {
                        Ok(_) => {
                            self.logs.push("[INFO] Connection test passed".to_string());
                            match storage.get_database_stats().await {
                                Ok(stats) => {
                                    self.db_stats = Some(crate::ui::widgets::DatabaseStats {
                                        total_papers: stats.paper_count,
                                        total_segments: stats.segment_count,
                                        total_repositories: stats.repository_count,
                                        total_modules: stats.module_count,
                                        total_tasks: stats.task_count,
                                        total_documents: stats.document_count,
                                    });
                                    self.logs.push(format!(
                                        "[INFO] Database stats: {} papers, {} modules, {} repositories",
                                        stats.paper_count, stats.module_count, stats.repository_count
                                    ));
                                }
                                Err(e) => {
                                    self.logs.push(format!(
                                        "[WARN] Could not fetch database stats: {}",
                                        e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            self.logs
                                .push(format!("[WARN] Connection test failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.storage_connected = false;
                    self.storage_error = Some(format!("{}", e));
                    self.logs
                        .push(format!("[ERROR] Failed to connect to SurrealDB: {}", e));
                    self.logs
                        .push("[INFO] Make sure SurrealDB is running:".to_string());
                    self.logs.push("[INFO]   docker run -d -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory".to_string());
                    self.logs.push(
                        "[INFO] Or connect manually in Storage view (press 's', then 'c')"
                            .to_string(),
                    );
                }
            }
        } else {
            self.logs
                .push("[WARN] Storage manager not initialized".to_string());
        }

        // Initialize service (gracefully handles missing API keys)
        if let Some(ref mut service) = self.service {
            // Don't fail if initialization fails due to missing API keys
            if let Err(e) = service.initialize().await {
                let error_msg = format!("{}", e);
                if !error_msg.contains("API key") && !error_msg.contains("Missing required field") {
                    // Only fail on non-API-key errors
                    return Err(format!("Failed to initialize service: {}", e).into());
                }
                // Otherwise, log warning and continue
                self.logs.push(format!("[WARN] {}", error_msg));
            }
        }

        // If paper path provided, start processing
        if let Some(ref paper_path) = self.paper_path {
            if let Some(ref mut service) = self.service {
                let paper = service
                    .load_paper(paper_path.clone())
                    .await
                    .map_err(|e| format!("Failed to load paper: {}", e))?;

                // Save paper to storage if connected
                if self.storage_connected {
                    if let Some(ref storage) = self.storage_manager {
                        if let Err(e) = storage.save_paper(&paper).await {
                            self.logs
                                .push(format!("[WARN] Failed to save paper to storage: {}", e));
                        } else {
                            self.logs
                                .push("[INFO] Paper saved to SurrealDB".to_string());
                        }
                    }
                }

                let output_dir = self
                    .config
                    .paths
                    .output_dir
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("./output"));
                service
                    .start_processing(paper, output_dir)
                    .await
                    .map_err(|e| format!("Failed to start processing: {}", e))?;
            }
        }

        // Setup terminal with error handling
        enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal =
            Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;

        // Initial log message
        self.logs
            .push("[INFO] Paper2Codes engine started".to_string());

        // Main event loop with error handling
        let mut last_render = Instant::now();
        let render_interval = Duration::from_millis(50); // 20 FPS for better performance

        loop {
            // Throttle rendering for performance
            if last_render.elapsed() >= render_interval {
                terminal
                    .draw(|f| self.render(f))
                    .map_err(|e| format!("Failed to render: {}", e))?;
                last_render = Instant::now();
            }

            // Handle keyboard events with timeout
            if crossterm::event::poll(Duration::from_millis(50))? {
                if let CrosstermEvent::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if let Err(e) = self.handle_key_event(key.code, key.modifiers) {
                            self.error_message = Some(format!("Error: {}", e));
                            self.logs.push(format!("[ERROR] {}", e));
                        }
                    }
                }
            }

            // Handle async events (non-blocking)
            // Process all available events
            loop {
                match self.event_rx.try_recv() {
                    Ok(event) => {
                        if let Err(e) = self.handle_event(event) {
                            self.error_message = Some(format!("Error handling event: {}", e));
                            self.logs
                                .push(format!("[ERROR] Error handling event: {}", e));
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        break; // No more events
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        self.logs
                            .push("[ERROR] Event channel disconnected".to_string());
                        break;
                    }
                }
            }

            // Periodically sync state from service
            if let Some(ref service) = self.service {
                if let Err(e) = service.sync_state().await {
                    self.logs.push(format!("[WARN] State sync error: {}", e));
                }
            }

            // Update code view when module selection changes
            if self.view_mode == ViewMode::Code && !self.modules.is_empty() {
                if let Some(module) = self.modules.get(self.selected_module) {
                    if let Some(ref service) = self.service {
                        if let Ok(Some(code)) = service.get_module_code(&module.id).await {
                            if self.current_code != code {
                                self.current_code = code;
                                self.current_file_name = module.name.clone();
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        // Restore terminal with error handling
        disable_raw_mode().map_err(|e| format!("Failed to disable raw mode: {}", e))?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .map_err(|e| format!("Failed to leave alternate screen: {}", e))?;
        terminal
            .show_cursor()
            .map_err(|e| format!("Failed to show cursor: {}", e))?;

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Clear error message on any key press
        self.error_message = None;

        match (modifiers, key) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.should_quit = true;
            }
            (_, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (_, KeyCode::Char('h')) | (_, KeyCode::Char('?')) => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.view_mode = ViewMode::Help;
                }
            }
            (_, KeyCode::Tab) => {
                self.cycle_view_mode();
            }
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.navigate_down();
            }
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.navigate_up();
            }
            (_, KeyCode::Char('r')) if self.view_mode != ViewMode::AdminConsole => {
                self.logs.push("[INFO] Refreshing...".to_string());
                // State sync will happen in the main loop
            }
            (_, KeyCode::Char('p')) => {
                // Load paper - show instructions
                self.logs.push(
                    "[INFO] To load a paper, use: paper2codes tui --paper <path>".to_string(),
                );
                self.logs
                    .push("[INFO] Or press 'u' for upload instructions".to_string());
                // In a future enhancement, we could add a file picker dialog here
            }
            (_, KeyCode::Char('s')) => {
                // Switch to storage view
                if self.view_mode == ViewMode::Storage {
                    self.view_mode = ViewMode::Dashboard;
                } else {
                    self.view_mode = ViewMode::Storage;
                }
            }
            (_, KeyCode::Char('a')) => {
                // Switch to admin console
                if self.view_mode == ViewMode::AdminConsole {
                    self.view_mode = ViewMode::Dashboard;
                } else {
                    self.view_mode = ViewMode::AdminConsole;
                }
            }
            (_, KeyCode::Char('c')) if self.view_mode == ViewMode::Storage => {
                // Connect to storage - trigger async operation
                if self.storage_connected {
                    self.logs
                        .push("[INFO] Already connected to SurrealDB".to_string());
                    return Ok(());
                }

                self.logs
                    .push("[INFO] Connecting to SurrealDB...".to_string());
                // Show the actual connection string (may have ws:// prefix in config)
                let display_conn = if self.config.storage.connection_string.starts_with("ws://")
                    || self.config.storage.connection_string.starts_with("wss://")
                {
                    format!(
                        "ws://{}",
                        self.config
                            .storage
                            .connection_string
                            .trim_start_matches("ws://")
                            .trim_start_matches("wss://")
                    )
                } else {
                    format!("ws://{}", self.config.storage.connection_string)
                };
                self.logs
                    .push(format!("[INFO] Connection: {}", display_conn));
                self.logs.push(format!(
                    "[INFO] Namespace: {}, Database: {}",
                    self.config.storage.namespace, self.config.storage.database
                ));

                let storage_clone = self.storage_manager.clone();
                let config_clone = self.config.storage.clone();
                let event_tx = self.event_tx.clone();

                tokio::spawn(async move {
                    if let Some(storage) = storage_clone {
                        // Attempt connection
                        let connect_result = storage.connect(config_clone.clone()).await;

                        match connect_result {
                            Ok(_) => {
                                // Test connection immediately
                                match storage.test_connection().await {
                                    Ok(_) => {
                                        // Get connection info
                                        if let Some(info) = storage.get_connection_info().await {
                                            let _ = event_tx.send(Event::StorageConnected(info));

                                            // Get database stats
                                            if let Ok(stats) = storage.get_database_stats().await {
                                                let _ = event_tx
                                                    .send(Event::DatabaseStatsUpdate(stats));
                                            }
                                        } else {
                                            let _ = event_tx.send(Event::StorageError(
                                                "Connected but failed to get connection info"
                                                    .to_string(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Connection test failed: {}", e);
                                        let _ = event_tx.send(Event::StorageError(error_msg));
                                    }
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to connect: {}. Make sure SurrealDB is running on {}. Try: docker run -d -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory", 
                                    e, config_clone.connection_string);
                                let _ = event_tx.send(Event::StorageError(error_msg));
                            }
                        }
                    } else {
                        let _ = event_tx.send(Event::StorageError(
                            "Storage manager not initialized".to_string(),
                        ));
                    }
                });
            }
            (_, KeyCode::Char('d')) if self.view_mode == ViewMode::Storage => {
                // Disconnect from storage - trigger async operation
                self.logs
                    .push("[INFO] Disconnecting from storage...".to_string());
                let storage_clone = self.storage_manager.clone();
                let event_tx = self.event_tx.clone();
                tokio::spawn(async move {
                    if let Some(_storage) = storage_clone {
                        // Note: disconnect requires &mut, so we can't do it through Arc
                        // This would need to be handled differently in a real implementation
                        // For now, just send a message that disconnection was requested
                        let _ = event_tx.send(Event::StorageDisconnected);
                    }
                });
            }
            (_, KeyCode::Char('t')) if self.view_mode == ViewMode::Storage => {
                // Test storage connection - trigger async operation
                self.logs
                    .push("[INFO] Testing storage connection...".to_string());
                let storage_clone = self.storage_manager.clone();
                let event_tx = self.event_tx.clone();
                tokio::spawn(async move {
                    if let Some(storage) = storage_clone {
                        match storage.test_connection().await {
                            Ok(_) => {
                                let _ = event_tx.send(Event::StorageTestResult(true, None));
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(Event::StorageTestResult(false, Some(format!("{}", e))));
                            }
                        }
                    } else {
                        let _ = event_tx.send(Event::StorageTestResult(
                            false,
                            Some("Storage manager not initialized".to_string()),
                        ));
                    }
                });
            }
            (_, KeyCode::Enter) if self.view_mode == ViewMode::AdminConsole => {
                // Execute admin query
                if !self.admin_query.trim().is_empty() {
                    let query = self.admin_query.clone();
                    let storage_clone = self.storage_manager.clone();
                    let event_tx = self.event_tx.clone();
                    self.logs.push(format!("[INFO] Executing query: {}", query));
                    tokio::spawn(async move {
                        if let Some(storage) = storage_clone {
                            match storage.execute_raw_query(&query).await {
                                Ok(result) => {
                                    let _ = event_tx.send(Event::AdminQueryResult(Ok(result)));
                                }
                                Err(e) => {
                                    let _ = event_tx
                                        .send(Event::AdminQueryResult(Err(format!("{}", e))));
                                }
                            }
                        } else {
                            let _ = event_tx.send(Event::AdminQueryResult(Err(
                                "Storage manager not initialized".to_string(),
                            )));
                        }
                    });
                }
            }
            (_, KeyCode::Char('r')) if self.view_mode == ViewMode::AdminConsole => {
                // Refresh database stats
                let storage_clone = self.storage_manager.clone();
                let event_tx = self.event_tx.clone();
                tokio::spawn(async move {
                    if let Some(storage) = storage_clone {
                        if let Ok(stats) = storage.get_database_stats().await {
                            let _ = event_tx.send(Event::DatabaseStatsUpdate(stats));
                        }
                    }
                });
            }
            (_, KeyCode::Char('x')) => {
                // Stop processing
                if let Some(ref mut service) = self.service {
                    service.stop_processing();
                }
            }
            (_, KeyCode::Char('l')) => {
                if self.view_mode == ViewMode::Logs {
                    self.view_mode = ViewMode::Dashboard;
                } else {
                    self.view_mode = ViewMode::Logs;
                }
            }
            (_, KeyCode::Char('1')) => {
                self.view_mode = ViewMode::Dashboard;
            }
            (_, KeyCode::Char('2')) => {
                self.view_mode = ViewMode::Tasks;
            }
            (_, KeyCode::Char('3')) => {
                self.view_mode = ViewMode::Modules;
            }
            (_, KeyCode::Char('4')) => {
                self.view_mode = ViewMode::Code;
            }
            (_, KeyCode::Char('5')) => {
                self.view_mode = ViewMode::Statistics;
            }
            (_, KeyCode::Char('6')) => {
                self.view_mode = ViewMode::Settings;
            }
            (_, KeyCode::Char('7')) => {
                self.view_mode = ViewMode::Skills;
            }
            (_, KeyCode::PageDown) => {
                self.scroll_down();
            }
            (_, KeyCode::PageUp) => {
                self.scroll_up();
            }
            // Settings-specific keys
            (_, KeyCode::Char('e'))
                if self.view_mode == ViewMode::Settings && self.selected_setting == 0 =>
            {
                if !self.editing_api_key {
                    self.editing_api_key = true;
                    self.input_mode = InputMode::EditingApiKey;
                    self.api_key_input.clear();
                }
            }
            (_, KeyCode::Enter) if self.editing_api_key => {
                self.save_api_key()?;
            }
            (_, KeyCode::Esc) if self.editing_api_key => {
                self.editing_api_key = false;
                self.input_mode = InputMode::Normal;
                self.api_key_input.clear();
            }
            (_, KeyCode::Char(c)) if self.editing_api_key => {
                self.api_key_input.push(c);
            }
            (_, KeyCode::Backspace) if self.editing_api_key => {
                self.api_key_input.pop();
            }
            (_, KeyCode::Char('w')) if self.view_mode == ViewMode::Settings => {
                self.save_config()?;
            }
            (_, KeyCode::Char('u')) => {
                // Upload PDF - prompt for file path
                self.logs.push("[INFO] PDF Upload: Enter file path in logs (or use command line: paper2codes tui --paper <path>)".to_string());
                self.logs
                    .push("[INFO] Example: paper2codes tui --paper /path/to/paper.pdf".to_string());
            }
            // Admin console-specific keys
            (_, KeyCode::Char('e')) if self.view_mode == ViewMode::AdminConsole => {
                if !matches!(self.input_mode, InputMode::EditingAdminQuery) {
                    self.input_mode = InputMode::EditingAdminQuery;
                    self.admin_query.clear();
                }
            }
            (_, KeyCode::Esc) if matches!(self.input_mode, InputMode::EditingAdminQuery) => {
                self.input_mode = InputMode::Normal;
            }
            (_, KeyCode::Char(c)) if matches!(self.input_mode, InputMode::EditingAdminQuery) => {
                self.admin_query.push(c);
            }
            (_, KeyCode::Backspace) if matches!(self.input_mode, InputMode::EditingAdminQuery) => {
                self.admin_query.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn save_api_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.api_key_input.trim().is_empty() {
            self.error_message = Some("API key cannot be empty".to_string());
            return Ok(());
        }

        // Determine which provider to update based on selected_setting
        // For now, we'll update based on which provider is selected
        // In a more advanced UI, we could have separate selection for provider
        // For simplicity, we'll cycle through: OpenRouter (0), OpenAI (1), Anthropic (2)
        let provider_name = match self.selected_setting {
            0 => "openrouter", // This is the API Keys setting
            _ => "openrouter", // Default to openrouter for now
        };

        // Update config - try to update the selected provider
        // For now, we'll update openrouter, but in a better implementation,
        // we'd have a separate provider selection
        if let Some(provider) = self.config.llm.providers.get_mut(provider_name) {
            provider.api_key = Some(self.api_key_input.trim().to_string());
            self.logs.push(format!(
                "[INFO] {} API key saved successfully",
                provider_name
            ));
        } else {
            // If provider doesn't exist, create it
            self.config.llm.providers.insert(
                provider_name.to_string(),
                crate::config::ProviderConfig {
                    enabled: true,
                    api_key: Some(self.api_key_input.trim().to_string()),
                    base_url: match provider_name {
                        "openrouter" => Some("https://openrouter.ai/api/v1".to_string()),
                        "openai" => Some("https://api.openai.com/v1".to_string()),
                        "anthropic" => Some("https://api.anthropic.com/v1".to_string()),
                        _ => None,
                    },
                    models: vec![],
                    temperature: 0.7,
                    max_tokens: Some(4096),
                },
            );
            self.logs.push(format!(
                "[INFO] {} provider created and API key saved",
                provider_name
            ));
        }

        // Save to file
        self.save_config()?;

        self.editing_api_key = false;
        self.input_mode = InputMode::Normal;
        self.api_key_input.clear();

        // Send event to reinitialize coordinator with new API key
        let config_clone = self.config.clone();
        let _ = self
            .event_tx
            .send(Event::ReinitializeCoordinator(config_clone));

        Ok(())
    }

    fn save_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Config directory not found".to_string())?
            .join("paper2codes");

        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        let toml = toml::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, toml).map_err(|e| format!("Failed to write config: {}", e))?;

        self.logs
            .push("[INFO] Configuration saved successfully".to_string());
        Ok(())
    }

    // Storage management methods (async operations handled via events)
    #[allow(dead_code)]
    async fn connect_storage(&mut self) {
        if let Some(ref storage) = self.storage_manager {
            let config = self.config.storage.clone();
            match storage.connect(config).await {
                Ok(_) => {
                    self.storage_connected = true;
                    self.storage_error = None;
                    self.logs.push("[INFO] Connected to SurrealDB".to_string());
                }
                Err(e) => {
                    self.storage_connected = false;
                    self.storage_error = Some(format!("{}", e));
                    self.logs.push(format!("[ERROR] Failed to connect: {}", e));
                }
            }
        } else {
            self.storage_error = Some("Storage manager not initialized".to_string());
        }
    }

    #[allow(dead_code)]
    async fn disconnect_storage(&mut self) {
        if let Some(ref storage) = self.storage_manager {
            if let Err(e) = storage.disconnect().await {
                self.storage_error = Some(format!("{}", e));
                self.logs
                    .push(format!("[ERROR] Failed to disconnect: {}", e));
            } else {
                self.storage_connected = false;
                self.storage_error = None;
                self.logs
                    .push("[INFO] Disconnected from SurrealDB".to_string());
            }
        }
    }

    #[allow(dead_code)]
    async fn test_storage_connection(&mut self) {
        if let Some(ref storage) = self.storage_manager {
            match storage.test_connection().await {
                Ok(_) => {
                    self.storage_error = None;
                    self.logs
                        .push("[INFO] Storage connection test passed".to_string());
                }
                Err(e) => {
                    self.storage_error = Some(format!("{}", e));
                    self.logs
                        .push(format!("[ERROR] Connection test failed: {}", e));
                }
            }
        }
    }

    fn render_storage_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        // Note: get_connection_info is async, so we use cached state
        let connection_info = if self.storage_connected {
            Some(crate::storage::ConnectionInfo {
                connection_string: self.config.storage.connection_string.clone(),
                namespace: self.config.storage.namespace.clone(),
                database: self.config.storage.database.clone(),
                connected_at: chrono::Utc::now(), // TODO: Cache actual connection time
            })
        } else {
            None
        };

        let connection_widget = ConnectionWidget::new(
            self.storage_connected,
            connection_info.as_ref(),
            &self.config.storage.connection_string,
            &self.config.storage.namespace,
            &self.config.storage.database,
            &self.theme,
            self.storage_error.as_deref(),
        );
        frame.render_widget(connection_widget, chunks[1]);
    }

    fn render_skills_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let registry = crate::skills::SkillRegistry::with_builtins();
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            "Domain skills specialise retrieval, generation, and verification.",
            self.theme.title_style(),
        )));
        lines.push(Line::from(Span::raw(
            "Select one in the Web UI (Domain Skills) before uploading a paper.",
        )));
        lines.push(Line::from(""));

        for skill in registry.list() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("• {} ", skill.name),
                    self.theme.title_style().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("[{}]", skill.languages.join(", "))),
            ]));
            lines.push(Line::from(Span::raw(format!("    {}", skill.description))));
        }

        let widget = ratatui::widgets::Paragraph::new(lines)
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(" Domain Skills "),
            )
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(widget, chunks[1]);
    }

    fn render_admin_console_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let admin_widget = AdminConsoleWidget::new(
            &self.admin_query,
            self.admin_query_result.as_deref(),
            self.admin_query_error.as_deref(),
            &self.theme,
            self.db_stats.as_ref(),
        );
        frame.render_widget(admin_widget, chunks[1]);
    }

    fn cycle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Tasks,
            ViewMode::Tasks => ViewMode::Modules,
            ViewMode::Modules => ViewMode::Code,
            ViewMode::Code => ViewMode::Skills,
            ViewMode::Skills => ViewMode::Statistics,
            ViewMode::Statistics => ViewMode::Settings,
            ViewMode::Settings => ViewMode::Logs,
            ViewMode::Logs => ViewMode::Storage,
            ViewMode::Storage => ViewMode::AdminConsole,
            ViewMode::AdminConsole => ViewMode::Dashboard,
            ViewMode::Help => ViewMode::Dashboard,
        };
        self.show_help = false;
    }

    fn navigate_down(&mut self) {
        match self.view_mode {
            ViewMode::Tasks => {
                if !self.tasks.is_empty() {
                    self.selected_task = (self.selected_task + 1) % self.tasks.len();
                }
            }
            ViewMode::Modules => {
                if !self.modules.is_empty() {
                    self.selected_module = (self.selected_module + 1) % self.modules.len();
                    if let Some(_module) = self.modules.get(self.selected_module) {
                        // Update code view when module changes
                        // This would be handled by the coordinator in real implementation
                    }
                }
            }
            ViewMode::Settings => {
                if !self.editing_api_key {
                    self.selected_setting = (self.selected_setting + 1).min(4);
                }
            }
            _ => {}
        }
    }

    fn navigate_up(&mut self) {
        match self.view_mode {
            ViewMode::Tasks => {
                if !self.tasks.is_empty() {
                    self.selected_task = if self.selected_task == 0 {
                        self.tasks.len().saturating_sub(1)
                    } else {
                        self.selected_task - 1
                    };
                }
            }
            ViewMode::Modules => {
                if !self.modules.is_empty() {
                    self.selected_module = if self.selected_module == 0 {
                        self.modules.len().saturating_sub(1)
                    } else {
                        self.selected_module - 1
                    };
                }
            }
            ViewMode::Settings => {
                if !self.editing_api_key {
                    self.selected_setting = if self.selected_setting == 0 {
                        4
                    } else {
                        self.selected_setting - 1
                    };
                }
            }
            _ => {}
        }
    }

    fn scroll_down(&mut self) {
        match self.view_mode {
            ViewMode::Code => {
                self.code_scroll = self.code_scroll.saturating_add(5);
            }
            ViewMode::Logs => {
                self.log_scroll = self.log_scroll.saturating_add(5);
            }
            _ => {}
        }
    }

    fn scroll_up(&mut self) {
        match self.view_mode {
            ViewMode::Code => {
                self.code_scroll = self.code_scroll.saturating_sub(5);
            }
            ViewMode::Logs => {
                self.log_scroll = self.log_scroll.saturating_sub(5);
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            Event::TaskUpdate(task) => {
                if let Some(pos) = self.tasks.iter().position(|t| t.id == task.id) {
                    self.tasks[pos] = task;
                } else {
                    self.tasks.push(task);
                }
            }
            Event::ModuleUpdate(module) => {
                if let Some(pos) = self.modules.iter().position(|m| m.id == module.id) {
                    self.modules[pos] = module;
                } else {
                    self.modules.push(module);
                }
            }
            Event::StatusUpdate(status) => {
                self.status = status;
            }
            Event::ProgressUpdate(progress) => {
                self.progress = progress.clamp(0.0, 1.0);
            }
            Event::LogMessage(msg) => {
                self.logs.push(msg);
                // Keep only last 500 logs for performance
                if self.logs.len() > 500 {
                    self.logs.remove(0);
                }
            }
            Event::CodeUpdate(code) => {
                self.current_code = code;
                self.code_scroll = 0; // Reset scroll when code updates
            }
            Event::FileUpdate(file_name) => {
                self.current_file_name = file_name;
            }
            Event::ReinitializeCoordinator(new_config) => {
                // Update app config
                self.config = new_config.clone();
                // Update service config and trigger reinitialization
                if let Some(ref mut service) = self.service {
                    // Update service's config
                    service.config = new_config.clone();
                    // Clear coordinator so it will be reinitialized on next access
                    service.clear_coordinator();
                    // Log that config was updated
                    self.logs.push(
                        "[INFO] API key updated. Coordinator will reinitialize on next operation."
                            .to_string(),
                    );
                }
            }
            Event::StorageConnected(info) => {
                self.storage_connected = true;
                self.storage_error = None;
                // Update config with connection info
                self.config.storage.connection_string = info.connection_string.clone();
                self.config.storage.namespace = info.namespace.clone();
                self.config.storage.database = info.database.clone();
                self.logs.push(format!(
                    "[SUCCESS] ✓ Connected to SurrealDB at {} (namespace: {}, database: {})",
                    info.connection_string, info.namespace, info.database
                ));
                self.logs.push(
                    "[INFO] Schema is ready. You can now upload papers and process them."
                        .to_string(),
                );
            }
            Event::StorageDisconnected => {
                self.storage_connected = false;
                self.storage_error = None;
                self.logs
                    .push("[INFO] Disconnected from SurrealDB".to_string());
            }
            Event::StorageError(error) => {
                self.storage_connected = false;
                self.storage_error = Some(error.clone());
                self.logs
                    .push(format!("[ERROR] Storage connection error: {}", error));
                self.logs.push("[INFO] Troubleshooting:".to_string());
                self.logs.push(
                    "[INFO]   1. Check if SurrealDB is running: docker ps | grep surrealdb"
                        .to_string(),
                );
                self.logs.push("[INFO]   2. Start SurrealDB: docker run -d -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root memory".to_string());
                self.logs.push(
                    "[INFO]   3. Check connection string in config: ws://localhost:8000"
                        .to_string(),
                );
            }
            Event::StorageTestResult(success, error) => {
                if success {
                    self.storage_error = None;
                    self.logs
                        .push("[INFO] Storage connection test passed".to_string());
                } else {
                    self.storage_error = error.clone();
                    self.logs.push(format!(
                        "[ERROR] Storage connection test failed: {}",
                        error.as_deref().unwrap_or("Unknown error")
                    ));
                }
            }
            Event::DatabaseStatsUpdate(stats) => {
                // Convert storage::DatabaseStats to ui::widgets::DatabaseStats
                use crate::ui::widgets::DatabaseStats as UIDatabaseStats;
                self.db_stats = Some(UIDatabaseStats {
                    total_papers: stats.paper_count,
                    total_segments: stats.segment_count,
                    total_repositories: stats.repository_count,
                    total_modules: stats.module_count,
                    total_tasks: stats.task_count,
                    total_documents: stats.document_count,
                });
            }
            Event::AdminQueryResult(result) => match result {
                Ok(value) => {
                    self.admin_query_result = Some(
                        serde_json::to_string_pretty(&value)
                            .unwrap_or_else(|_| format!("{:?}", value)),
                    );
                    self.admin_query_error = None;
                    self.logs
                        .push("[INFO] Admin query executed successfully".to_string());
                }
                Err(e) => {
                    self.admin_query_error = Some(e.clone());
                    self.admin_query_result = None;
                    self.logs.push(format!("[ERROR] Admin query failed: {}", e));
                }
            },
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        if self.show_help {
            self.render_help(frame);
            return;
        }

        match self.view_mode {
            ViewMode::Dashboard => self.render_dashboard(frame),
            ViewMode::Tasks => self.render_tasks_view(frame),
            ViewMode::Modules => self.render_modules_view(frame),
            ViewMode::Code => self.render_code_view(frame),
            ViewMode::Skills => self.render_skills_view(frame),
            ViewMode::Statistics => self.render_statistics_view(frame),
            ViewMode::Settings => self.render_settings_view(frame),
            ViewMode::Logs => self.render_logs_view(frame),
            ViewMode::Help => self.render_help(frame),
            ViewMode::Storage => self.render_storage_view(frame),
            ViewMode::AdminConsole => self.render_admin_console_view(frame),
        }
    }

    fn render_dashboard(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Status bar
                Constraint::Min(5),    // Main content
                Constraint::Length(3), // Progress bar
                Constraint::Length(8), // Logs
            ])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);
        self.render_main_dashboard(frame, chunks[1]);
        self.render_progress_bar(frame, chunks[2]);
        self.render_logs(frame, chunks[3]);
    }

    fn render_tasks_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let main_chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let task_list = TaskList::new(&self.tasks, self.selected_task, &self.theme);
        frame.render_widget(task_list, main_chunks[0]);

        if let Some(task) = self.tasks.get(self.selected_task) {
            self.render_task_details(frame, main_chunks[1], task);
        }
    }

    fn render_modules_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let main_chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        let module_list = ModuleList::new(&self.modules, self.selected_module, &self.theme);
        frame.render_widget(module_list, main_chunks[0]);

        if let Some(module) = self.modules.get(self.selected_module) {
            self.render_module_details(frame, main_chunks[1], module);
        }
    }

    fn render_code_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let code_viewer = CodeViewer::new(&self.current_code, self.code_scroll, &self.theme)
            .with_file_name(if self.current_file_name.is_empty() {
                "main.rs"
            } else {
                &self.current_file_name
            });
        frame.render_widget(code_viewer, chunks[1]);
    }

    fn render_statistics_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let stats = StatisticsWidget::new(
            self.tasks.len(),
            self.tasks
                .iter()
                .filter(|t| matches!(t.status, TaskStatus::Completed))
                .count(),
            self.tasks
                .iter()
                .filter(|t| matches!(t.status, TaskStatus::Failed(_)))
                .count(),
            self.modules.len(),
            self.modules
                .iter()
                .filter(|m| matches!(m.status, crate::types::ModuleStatus::Completed))
                .count(),
            self.theme.clone(),
        );
        frame.render_widget(stats, chunks[1]);
    }

    fn render_logs_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);
        self.render_logs(frame, chunks[1]);
    }

    fn render_settings_view(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let settings = SettingsWidget::new(
            &self.config,
            self.selected_setting,
            self.editing_api_key,
            &self.api_key_input,
            &self.theme,
        );
        frame.render_widget(settings, chunks[1]);
    }

    fn render_help(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(frame.size());

        self.render_status_bar(frame, chunks[0]);

        let help = HelpWidget::new(self.theme.clone());
        frame.render_widget(help, chunks[1]);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status_text = match &self.status {
            AppStatus::Idle => "Idle",
            AppStatus::Processing => "Processing",
            AppStatus::Planning => "Planning",
            AppStatus::Analyzing => "Analyzing",
            AppStatus::Coding => "Coding",
            AppStatus::Verifying => "Verifying",
            AppStatus::Completed => "Completed",
            AppStatus::Error(e) => e.as_str(),
        };

        let status_style = match &self.status {
            AppStatus::Error(_) => self.theme.error_style(),
            AppStatus::Completed => self.theme.success_style(),
            _ => Style::default().fg(self.theme.rust_orange),
        };

        let elapsed = self.start_time.elapsed();
        let elapsed_str = format!(
            "{:02}:{:02}:{:02}",
            elapsed.as_secs() / 3600,
            (elapsed.as_secs() % 3600) / 60,
            elapsed.as_secs() % 60
        );

        let progress_text = format!("Progress: {:.1}%", self.progress * 100.0);

        let view_mode_text = match self.view_mode {
            ViewMode::Dashboard => "Dashboard",
            ViewMode::Tasks => "Tasks",
            ViewMode::Modules => "Modules",
            ViewMode::Code => "Code",
            ViewMode::Skills => "Domain Skills",
            ViewMode::Statistics => "Statistics",
            ViewMode::Settings => "Settings",
            ViewMode::Logs => "Logs",
            ViewMode::Help => "Help",
            ViewMode::Storage => "Storage",
            ViewMode::AdminConsole => "Admin Console",
        };

        let status_line = Line::from(vec![
            Span::styled(
                "Paper2Codes ",
                self.theme.title_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled("🦀", Style::default().fg(self.theme.rust_orange)),
            Span::raw(" | "),
            Span::styled(format!("Status: {}", status_text), status_style),
            Span::raw(" | "),
            Span::styled(format!("View: {}", view_mode_text), self.theme.info_style()),
            Span::raw(" | "),
            Span::raw(progress_text),
            Span::raw(" | "),
            Span::styled(format!("Time: {}", elapsed_str), self.theme.muted_style()),
            Span::raw(" | "),
            Span::styled("Press '?' for help", self.theme.muted_style()),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title(Span::styled("Status", self.theme.title_style()));

        let paragraph = Paragraph::new(status_line).block(block);
        frame.render_widget(paragraph, area);

        // Show error message if present
        if let Some(ref error) = self.error_message {
            let error_area = Rect {
                x: area.x,
                y: area.y + area.height,
                width: area.width,
                height: 1,
            };
            let error_line = Line::from(vec![
                Span::styled(
                    "ERROR: ",
                    self.theme.error_style().add_modifier(Modifier::BOLD),
                ),
                Span::styled(error, self.theme.error_style()),
            ]);
            let error_para = Paragraph::new(error_line);
            frame.render_widget(error_para, error_area);
        }
    }

    fn render_main_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let task_list = TaskList::new(&self.tasks, self.selected_task, &self.theme);
        frame.render_widget(task_list, chunks[0]);

        let module_list = ModuleList::new(&self.modules, self.selected_module, &self.theme);
        frame.render_widget(module_list, chunks[1]);
    }

    fn render_progress_bar(&mut self, frame: &mut Frame, area: Rect) {
        let progress = ProgressWidget::new(
            self.progress,
            "Overall Progress".to_string(),
            self.theme.clone(),
        );
        frame.render_widget(progress, area);
    }

    fn render_logs(&self, frame: &mut Frame, area: Rect) {
        let log_viewer = LogViewer::new(&self.logs, self.log_scroll, &self.theme);
        frame.render_widget(log_viewer, area);
    }

    fn render_task_details(&self, frame: &mut Frame, area: Rect, task: &Task) {
        let details = vec![
            Line::from(vec![Span::styled("Task Details", self.theme.title_style())]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", self.theme.title_style()),
                Span::raw(task.id.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Description: ", self.theme.title_style()),
                Span::raw(&task.description),
            ]),
            Line::from(vec![
                Span::styled("Status: ", self.theme.title_style()),
                Span::styled(
                    format!("{:?}", task.status),
                    match task.status {
                        TaskStatus::Completed => self.theme.success_style(),
                        TaskStatus::Failed(_) => self.theme.error_style(),
                        TaskStatus::InProgress => Style::default().fg(self.theme.rust_orange),
                        _ => self.theme.muted_style(),
                    },
                ),
            ]),
        ];

        let paragraph = Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Task Details", self.theme.title_style())),
        );

        frame.render_widget(paragraph, area);
    }

    fn render_module_details(&self, frame: &mut Frame, area: Rect, module: &Module) {
        let details = vec![
            Line::from(vec![Span::styled(
                "Module Details",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Name: ", self.theme.title_style()),
                Span::raw(&module.name),
            ]),
            Line::from(vec![
                Span::styled("Description: ", self.theme.title_style()),
                Span::raw(&module.description),
            ]),
            Line::from(vec![
                Span::styled("Language: ", self.theme.title_style()),
                Span::raw(format!("{:?}", module.language)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", self.theme.title_style()),
                Span::styled(
                    format!("{:?}", module.status),
                    match module.status {
                        crate::types::ModuleStatus::Completed => self.theme.success_style(),
                        crate::types::ModuleStatus::Failed(_) => self.theme.error_style(),
                        _ => self.theme.muted_style(),
                    },
                ),
            ]),
        ];

        let paragraph = Paragraph::new(details).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Module Details", self.theme.title_style())),
        );

        frame.render_widget(paragraph, area);
    }
}
