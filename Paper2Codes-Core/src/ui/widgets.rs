use crate::config::Config;
use crate::types::{Module, ModuleStatus, Task, TaskStatus};
use crate::ui::theme::Theme;
use ratatui::layout::Layout;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Table};

pub struct TaskList<'a> {
    tasks: &'a [Task],
    selected: usize,
    theme: &'a Theme,
}

impl<'a> TaskList<'a> {
    pub fn new(tasks: &'a [Task], selected: usize, theme: &'a Theme) -> Self {
        Self {
            tasks,
            selected,
            theme,
        }
    }
}

impl<'a> ratatui::widgets::Widget for TaskList<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let items: Vec<ListItem> = self
            .tasks
            .iter()
            .map(|task| {
                let (status_icon, status_color): (&str, Color) = match task.status {
                    TaskStatus::Pending => ("○", Color::DarkGray),
                    TaskStatus::InProgress => ("→", self.theme.rust_orange),
                    TaskStatus::Completed => ("✓", self.theme.success),
                    TaskStatus::Failed(_) => ("✗", self.theme.error),
                };

                let task_type = match &task.task_type {
                    crate::types::TaskType::Planning => ("Plan", Color::Cyan),
                    crate::types::TaskType::Analysis { .. } => ("Analyze", Color::Blue),
                    crate::types::TaskType::Coding { .. } => ("Code", self.theme.rust_orange),
                    crate::types::TaskType::Verification { .. } => ("Verify", Color::Green),
                    crate::types::TaskType::Fix { .. } => ("Fix", Color::Yellow),
                };

                let description = if task.description.len() > 40 {
                    format!("{}...", &task.description[..40])
                } else {
                    task.description.clone()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        format!("[{}] ", task_type.0),
                        Style::default().fg(task_type.1),
                    ),
                    Span::raw(description),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Tasks", self.theme.title_style())),
            )
            .highlight_style(self.theme.highlight_style())
            .highlight_symbol("▶ ");

        let mut state = ListState::default();
        state.select(Some(self.selected));

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut state);
    }
}

pub struct CodeViewer<'a> {
    code: &'a str,
    scroll: usize,
    theme: &'a Theme,
    file_name: Option<&'a str>,
}

impl<'a> CodeViewer<'a> {
    pub fn new(code: &'a str, scroll: usize, theme: &'a Theme) -> Self {
        Self {
            code,
            scroll,
            theme,
            file_name: None,
        }
    }

    pub fn with_file_name(mut self, file_name: &'a str) -> Self {
        self.file_name = Some(file_name);
        self
    }
}

impl<'a> ratatui::widgets::Widget for CodeViewer<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let lines: Vec<Line> = self
            .code
            .lines()
            .enumerate()
            .skip(self.scroll)
            .take(area.height as usize - 2)
            .map(|(idx, line)| {
                let line_num = idx + 1;
                let line_num_str = format!("{:4} │ ", line_num);
                Line::from(vec![
                    Span::styled(
                        line_num_str,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ),
                    Span::styled(line, self.theme.code_style()),
                ])
            })
            .collect();

        let title = if let Some(name) = self.file_name {
            format!("Code Viewer - {}", name)
        } else {
            "Code Viewer".to_string()
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled(title, self.theme.title_style())),
            )
            .style(self.theme.code_style())
            .scroll((self.scroll as u16, 0));

        paragraph.render(area, buf);
    }
}

pub struct LogViewer<'a> {
    logs: &'a [String],
    scroll: usize,
    theme: &'a Theme,
}

impl<'a> LogViewer<'a> {
    pub fn new(logs: &'a [String], scroll: usize, theme: &'a Theme) -> Self {
        Self {
            logs,
            scroll,
            theme,
        }
    }
}

impl<'a> ratatui::widgets::Widget for LogViewer<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .skip(self.scroll)
            .take(area.height as usize - 2)
            .map(|log| {
                let (level_color, level_text) = if log.starts_with("[ERROR]") {
                    (self.theme.error, "[ERROR]")
                } else if log.starts_with("[WARN]") {
                    (self.theme.warning, "[WARN]")
                } else if log.starts_with("[INFO]") {
                    (self.theme.info, "[INFO]")
                } else if log.starts_with("[DEBUG]") {
                    (Color::DarkGray, "[DEBUG]")
                } else {
                    (self.theme.foreground, "")
                };

                let message = log
                    .strip_prefix(level_text)
                    .unwrap_or(log)
                    .strip_prefix("] ")
                    .unwrap_or(log);

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", level_text),
                        Style::default()
                            .fg(level_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(message, Style::default().fg(self.theme.foreground)),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Logs", self.theme.title_style())),
        );

        list.render(area, buf);
    }
}

pub struct ProgressWidget {
    progress: f32,
    label: String,
    theme: Theme,
}

impl ProgressWidget {
    pub fn new(progress: f32, label: String, theme: Theme) -> Self {
        Self {
            progress,
            label,
            theme,
        }
    }
}

impl ratatui::widgets::Widget for ProgressWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled(self.label, self.theme.title_style())),
            )
            .gauge_style(Style::default().fg(self.theme.rust_orange))
            .percent((self.progress * 100.0) as u16)
            .label(format!("{:.1}%", self.progress * 100.0));

        gauge.render(area, buf);
    }
}

pub struct ModuleList<'a> {
    modules: &'a [Module],
    selected: usize,
    theme: &'a Theme,
}

impl<'a> ModuleList<'a> {
    pub fn new(modules: &'a [Module], selected: usize, theme: &'a Theme) -> Self {
        Self {
            modules,
            selected,
            theme,
        }
    }
}

impl<'a> ratatui::widgets::Widget for ModuleList<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let items: Vec<ListItem> = self
            .modules
            .iter()
            .map(|module| {
                let (status_icon, status_color): (&str, Color) = match module.status {
                    ModuleStatus::Pending => ("○", Color::DarkGray),
                    ModuleStatus::Analyzing => ("→", self.theme.rust_orange),
                    ModuleStatus::Coding => ("⚙", self.theme.rust_orange),
                    ModuleStatus::Verifying => ("✓", self.theme.info),
                    ModuleStatus::Completed => ("✓", self.theme.success),
                    ModuleStatus::Failed(_) => ("✗", self.theme.error),
                };

                let lang_icon = match module.language {
                    crate::types::ProgrammingLanguage::Python => "🐍",
                    crate::types::ProgrammingLanguage::Rust => "🦀",
                    crate::types::ProgrammingLanguage::Other(_) => "📄",
                };

                let name = if module.name.len() > 30 {
                    format!("{}...", &module.name[..30])
                } else {
                    module.name.clone()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(format!("{} ", lang_icon)),
                    Span::styled(name, Style::default().fg(self.theme.foreground)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Modules", self.theme.title_style())),
            )
            .highlight_style(self.theme.highlight_style())
            .highlight_symbol("▶ ");

        let mut state = ListState::default();
        state.select(Some(self.selected));

        ratatui::widgets::StatefulWidget::render(list, area, buf, &mut state);
    }
}

pub struct StatisticsWidget {
    tasks_total: usize,
    tasks_completed: usize,
    tasks_failed: usize,
    modules_total: usize,
    modules_completed: usize,
    theme: Theme,
}

impl StatisticsWidget {
    pub fn new(
        tasks_total: usize,
        tasks_completed: usize,
        tasks_failed: usize,
        modules_total: usize,
        modules_completed: usize,
        theme: Theme,
    ) -> Self {
        Self {
            tasks_total,
            tasks_completed,
            tasks_failed,
            modules_total,
            modules_completed,
            theme,
        }
    }
}

impl ratatui::widgets::Widget for StatisticsWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let task_progress = if self.tasks_total > 0 {
            self.tasks_completed as f32 / self.tasks_total as f32
        } else {
            0.0
        };

        let module_progress = if self.modules_total > 0 {
            self.modules_completed as f32 / self.modules_total as f32
        } else {
            0.0
        };

        let rows = vec![
            vec![
                "Tasks".to_string(),
                format!("{}/{}", self.tasks_completed, self.tasks_total),
            ],
            vec!["Failed".to_string(), format!("{}", self.tasks_failed)],
            vec![
                "Modules".to_string(),
                format!("{}/{}", self.modules_completed, self.modules_total),
            ],
            vec![
                "Task Progress".to_string(),
                format!("{:.1}%", task_progress * 100.0),
            ],
            vec![
                "Module Progress".to_string(),
                format!("{:.1}%", module_progress * 100.0),
            ],
        ];

        let table = Table::new(
            rows.iter().map(|row| {
                ratatui::widgets::Row::new(
                    row.iter()
                        .map(|cell| ratatui::widgets::Cell::from(cell.as_str())),
                )
            }),
            [Constraint::Percentage(60), Constraint::Percentage(40)],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Statistics", self.theme.title_style())),
        )
        .column_spacing(1);

        table.render(area, buf);
    }
}

pub struct HelpWidget {
    theme: Theme,
}

impl HelpWidget {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }
}

impl ratatui::widgets::Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let help_text = vec![
            Line::from(vec![Span::styled("Navigation:", self.theme.title_style())]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("j/↓", self.theme.rust_orange),
                Span::raw(" / "),
                Span::styled("k/↑", self.theme.rust_orange),
                Span::raw(" - Navigate tasks/modules"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Tab", self.theme.rust_orange),
                Span::raw(" - Switch panels"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("h", self.theme.rust_orange),
                Span::raw(" - Show/hide help"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Views:", self.theme.title_style())]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("1", self.theme.rust_orange),
                Span::raw(" - Dashboard  "),
                Span::styled("2", self.theme.rust_orange),
                Span::raw(" - Tasks  "),
                Span::styled("3", self.theme.rust_orange),
                Span::raw(" - Modules"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("4", self.theme.rust_orange),
                Span::raw(" - Code  "),
                Span::styled("5", self.theme.rust_orange),
                Span::raw(" - Statistics"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("6", self.theme.rust_orange),
                Span::raw(" - Settings  "),
                Span::styled("l", self.theme.rust_orange),
                Span::raw(" - Logs"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Actions:", self.theme.title_style())]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("r", self.theme.rust_orange),
                Span::raw(" - Refresh"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("s", self.theme.rust_orange),
                Span::raw(" - Save state  "),
                Span::styled("w", self.theme.rust_orange),
                Span::raw(" - Save config (in Settings)"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("e", self.theme.rust_orange),
                Span::raw(" - Edit API key (in Settings)"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("l", self.theme.rust_orange),
                Span::raw(" - Toggle logs"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled("Other:", self.theme.title_style())]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("q", self.theme.rust_orange),
                Span::raw(" - Quit"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("?", self.theme.rust_orange),
                Span::raw(" - Show this help"),
            ]),
        ];

        let paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Help", self.theme.title_style())),
            )
            .style(self.theme.foreground);

        paragraph.render(area, buf);
    }
}

pub struct SettingsWidget<'a> {
    config: &'a Config,
    selected_setting: usize,
    editing_api_key: bool,
    api_key_input: &'a str,
    theme: &'a Theme,
}

impl<'a> SettingsWidget<'a> {
    pub fn new(
        config: &'a Config,
        selected_setting: usize,
        editing_api_key: bool,
        api_key_input: &'a str,
        theme: &'a Theme,
    ) -> Self {
        Self {
            config,
            selected_setting,
            editing_api_key,
            api_key_input,
            theme,
        }
    }
}

impl<'a> ratatui::widgets::Widget for SettingsWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Settings list
        let settings_items = vec![
            ListItem::new(Line::from(vec![
                Span::styled("🔑 ", self.theme.rust_orange),
                Span::raw("API Keys (OpenRouter/OpenAI/Anthropic)"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("🤖 ", self.theme.rust_orange),
                Span::raw("LLM Models"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("⚙️  ", self.theme.rust_orange),
                Span::raw("Agent Settings"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("✅ ", self.theme.rust_orange),
                Span::raw("Verification Settings"),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled("📁 ", self.theme.rust_orange),
                Span::raw("Paths"),
            ])),
        ];

        let settings_list = List::new(settings_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Settings", self.theme.title_style())),
            )
            .highlight_style(self.theme.highlight_style())
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_setting));
        ratatui::widgets::StatefulWidget::render(settings_list, chunks[0], buf, &mut list_state);

        // Settings details
        let details = match self.selected_setting {
            0 => self.render_api_key_settings(chunks[1]),
            1 => self.render_llm_settings(chunks[1]),
            2 => self.render_agent_settings(chunks[1]),
            3 => self.render_verification_settings(chunks[1]),
            4 => self.render_path_settings(chunks[1]),
            _ => Paragraph::new("").block(Block::default().borders(Borders::ALL)),
        };

        details.render(chunks[1], buf);
    }
}

impl<'a> SettingsWidget<'a> {
    fn render_api_key_settings(&self, _area: Rect) -> Paragraph<'_> {
        // Check status for each provider
        let openrouter_key = self
            .config
            .llm
            .providers
            .get("openrouter")
            .and_then(|p| p.api_key.as_ref())
            .map(|k| {
                if k.len() > 8 {
                    format!("{}...{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "***".to_string()
                }
            })
            .unwrap_or_else(|| "Not set".to_string());

        let openai_key = self
            .config
            .llm
            .providers
            .get("openai")
            .and_then(|p| p.api_key.as_ref())
            .map(|k| {
                if k.len() > 8 {
                    format!("{}...{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "***".to_string()
                }
            })
            .unwrap_or_else(|| "Not set".to_string());

        let anthropic_key = self
            .config
            .llm
            .providers
            .get("anthropic")
            .and_then(|p| p.api_key.as_ref())
            .map(|k| {
                if k.len() > 8 {
                    format!("{}...{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "***".to_string()
                }
            })
            .unwrap_or_else(|| "Not set".to_string());

        let openrouter_status = if self
            .config
            .llm
            .providers
            .get("openrouter")
            .and_then(|p| p.api_key.as_ref())
            .is_some()
        {
            Span::styled("✓ Set", self.theme.success_style())
        } else {
            Span::styled("✗ Not Set", self.theme.error_style())
        };

        let openai_status = if self
            .config
            .llm
            .providers
            .get("openai")
            .and_then(|p| p.api_key.as_ref())
            .is_some()
            || std::env::var("OPENAI_API_KEY").is_ok()
        {
            Span::styled("✓ Set", self.theme.success_style())
        } else {
            Span::styled("✗ Not Set", self.theme.error_style())
        };

        let anthropic_status = if self
            .config
            .llm
            .providers
            .get("anthropic")
            .and_then(|p| p.api_key.as_ref())
            .is_some()
            || std::env::var("ANTHROPIC_API_KEY").is_ok()
        {
            Span::styled("✓ Set", self.theme.success_style())
        } else {
            Span::styled("✗ Not Set", self.theme.error_style())
        };

        let input_display = if self.editing_api_key {
            if self.api_key_input.is_empty() {
                Span::styled("Enter API key...", self.theme.muted_style())
            } else {
                let masked = "*".repeat(self.api_key_input.len().min(50));
                Span::styled(masked, self.theme.foreground)
            }
        } else {
            Span::raw("Press 'e' to edit selected provider")
        };

        let content = vec![
            Line::from(vec![Span::styled(
                "API Key Configuration",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("OpenRouter: ", self.theme.title_style()),
                openrouter_status,
                Span::raw(" "),
                Span::styled(format!("({})", openrouter_key), self.theme.muted_style()),
            ]),
            Line::from("  https://openrouter.ai/keys"),
            Line::from(""),
            Line::from(vec![
                Span::styled("OpenAI: ", self.theme.title_style()),
                openai_status,
                Span::raw(" "),
                Span::styled(format!("({})", openai_key), self.theme.muted_style()),
            ]),
            Line::from("  https://platform.openai.com/api-keys"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Anthropic: ", self.theme.title_style()),
                anthropic_status,
                Span::raw(" "),
                Span::styled(format!("({})", anthropic_key), self.theme.muted_style()),
            ]),
            Line::from("  https://console.anthropic.com/settings/keys"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Instructions:",
                self.theme.title_style(),
            )]),
            Line::from("  1. Select a provider above (j/k to navigate)"),
            Line::from("  2. Press 'e' to edit"),
            Line::from("  3. Enter your API key"),
            Line::from("  4. Press Enter to save, Esc to cancel"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Input: ", self.theme.title_style()),
                input_display,
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Note: ", self.theme.warning_style()),
                Span::raw("API keys are stored in ~/.config/paper2codes/config.toml"),
            ]),
            Line::from(vec![
                Span::styled("      ", self.theme.warning_style()),
                Span::raw("You can also set via environment variables:"),
            ]),
            Line::from(vec![
                Span::styled("      ", self.theme.warning_style()),
                Span::raw("OPENROUTER_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY"),
            ]),
        ];

        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled(
                    "API Key Configuration",
                    self.theme.title_style(),
                )),
        )
    }

    fn render_llm_settings(&self, _area: Rect) -> Paragraph<'_> {
        let default_provider = &self.config.llm.default_provider;
        let models = self
            .config
            .llm
            .providers
            .get(default_provider)
            .map(|p| p.models.join(", "))
            .unwrap_or_else(|| "None".to_string());

        let content = vec![
            Line::from(vec![Span::styled(
                "LLM Configuration",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Default Provider: ", self.theme.title_style()),
                Span::raw(default_provider),
            ]),
            Line::from(vec![
                Span::styled("Available Models: ", self.theme.title_style()),
                Span::raw(models),
            ]),
            Line::from(vec![
                Span::styled("Timeout: ", self.theme.title_style()),
                Span::raw(format!("{}s", self.config.llm.timeout_seconds)),
            ]),
            Line::from(vec![
                Span::styled("Max Retries: ", self.theme.title_style()),
                Span::raw(format!("{}", self.config.llm.max_retries)),
            ]),
        ];

        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("LLM Settings", self.theme.title_style())),
        )
    }

    fn render_agent_settings(&self, _area: Rect) -> Paragraph<'_> {
        let content = vec![
            Line::from(vec![Span::styled(
                "Agent Configuration",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Planning Model: ", self.theme.title_style()),
                Span::raw(&self.config.agents.planning_model),
            ]),
            Line::from(vec![
                Span::styled("Analysis Model: ", self.theme.title_style()),
                Span::raw(&self.config.agents.analysis_model),
            ]),
            Line::from(vec![
                Span::styled("Coding Model: ", self.theme.title_style()),
                Span::raw(&self.config.agents.coding_model),
            ]),
            Line::from(vec![
                Span::styled("Verification Model: ", self.theme.title_style()),
                Span::raw(&self.config.agents.verification_model),
            ]),
            Line::from(vec![
                Span::styled("Max Iterations: ", self.theme.title_style()),
                Span::raw(format!("{}", self.config.agents.max_iterations)),
            ]),
            Line::from(vec![
                Span::styled("Parallel Tasks: ", self.theme.title_style()),
                Span::raw(format!("{}", self.config.agents.parallel_tasks)),
            ]),
        ];

        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Agent Settings", self.theme.title_style())),
        )
    }

    fn render_verification_settings(&self, _area: Rect) -> Paragraph<'_> {
        let content = vec![
            Line::from(vec![Span::styled(
                "Verification Configuration",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Static Checks: ", self.theme.title_style()),
                Span::styled(
                    if self.config.verification.enable_static_checks {
                        "✓ Enabled"
                    } else {
                        "✗ Disabled"
                    },
                    if self.config.verification.enable_static_checks {
                        self.theme.success_style()
                    } else {
                        self.theme.error_style()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Dynamic Tests: ", self.theme.title_style()),
                Span::styled(
                    if self.config.verification.enable_dynamic_tests {
                        "✓ Enabled"
                    } else {
                        "✗ Disabled"
                    },
                    if self.config.verification.enable_dynamic_tests {
                        self.theme.success_style()
                    } else {
                        self.theme.error_style()
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Symbolic Verification: ", self.theme.title_style()),
                Span::styled(
                    if self.config.verification.enable_symbolic_verification {
                        "✓ Enabled"
                    } else {
                        "✗ Disabled"
                    },
                    if self.config.verification.enable_symbolic_verification {
                        self.theme.success_style()
                    } else {
                        self.theme.error_style()
                    },
                ),
            ]),
        ];

        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled(
                    "Verification Settings",
                    self.theme.title_style(),
                )),
        )
    }

    fn render_path_settings(&self, _area: Rect) -> Paragraph<'_> {
        let config_dir = self
            .config
            .paths
            .config_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "Default".to_string());

        let output_dir = self
            .config
            .paths
            .output_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "./output".to_string());

        let content = vec![
            Line::from(vec![Span::styled(
                "Path Configuration",
                self.theme.title_style(),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Config Directory: ", self.theme.title_style()),
                Span::raw(config_dir),
            ]),
            Line::from(vec![
                Span::styled("Output Directory: ", self.theme.title_style()),
                Span::raw(output_dir),
            ]),
        ];

        Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.border_style())
                .title(Span::styled("Path Settings", self.theme.title_style())),
        )
    }
}

/// Connection Management Widget
pub struct ConnectionWidget<'a> {
    connected: bool,
    connection_info: Option<&'a crate::storage::ConnectionInfo>,
    connection_string: &'a str,
    namespace: &'a str,
    database: &'a str,
    theme: &'a Theme,
    error: Option<&'a str>,
}

impl<'a> ConnectionWidget<'a> {
    pub fn new(
        connected: bool,
        connection_info: Option<&'a crate::storage::ConnectionInfo>,
        connection_string: &'a str,
        namespace: &'a str,
        database: &'a str,
        theme: &'a Theme,
        error: Option<&'a str>,
    ) -> Self {
        Self {
            connected,
            connection_info,
            connection_string,
            namespace,
            database,
            theme,
            error,
        }
    }
}

impl<'a> ratatui::widgets::Widget for ConnectionWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let status_color = if self.connected {
            self.theme.success
        } else {
            Color::Red
        };
        let status_text = if self.connected {
            "Connected"
        } else {
            "Disconnected"
        };

        let mut content = vec![
            Line::from(vec![
                Span::styled("Status: ", self.theme.title_style()),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("Connection: ", self.theme.title_style()),
                Span::raw(self.connection_string),
            ]),
            Line::from(vec![
                Span::styled("Namespace: ", self.theme.title_style()),
                Span::raw(self.namespace),
            ]),
            Line::from(vec![
                Span::styled("Database: ", self.theme.title_style()),
                Span::raw(self.database),
            ]),
        ];

        if let Some(info) = self.connection_info {
            content.push(Line::from(vec![
                Span::styled("Connected at: ", self.theme.title_style()),
                Span::raw(info.connected_at.format("%Y-%m-%d %H:%M:%S").to_string()),
            ]));
        }

        if let Some(error) = self.error {
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::raw(error),
            ]));
        }

        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled(
                        "SurrealDB Connection",
                        self.theme.title_style(),
                    )),
            )
            .render(area, buf);
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_papers: usize,
    pub total_segments: usize,
    pub total_repositories: usize,
    pub total_modules: usize,
    pub total_tasks: usize,
    pub total_documents: usize,
}

/// Admin Console Widget for SurrealDB
pub struct AdminConsoleWidget<'a> {
    query: &'a str,
    query_result: Option<&'a str>,
    query_error: Option<&'a str>,
    theme: &'a Theme,
    stats: Option<&'a DatabaseStats>,
}

impl<'a> AdminConsoleWidget<'a> {
    pub fn new(
        query: &'a str,
        query_result: Option<&'a str>,
        query_error: Option<&'a str>,
        theme: &'a Theme,
        stats: Option<&'a DatabaseStats>,
    ) -> Self {
        Self {
            query,
            query_result,
            query_error,
            theme,
            stats,
        }
    }
}

impl<'a> ratatui::widgets::Widget for AdminConsoleWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Stats
                Constraint::Min(5),     // Query area
                Constraint::Length(10), // Results
            ])
            .split(area);

        // Stats section
        if let Some(stats) = self.stats {
            let stats_content = vec![
                Line::from(vec![
                    Span::styled("Papers: ", self.theme.title_style()),
                    Span::raw(stats.total_papers.to_string()),
                    Span::raw(" | "),
                    Span::styled("Segments: ", self.theme.title_style()),
                    Span::raw(stats.total_segments.to_string()),
                    Span::raw(" | "),
                    Span::styled("Repos: ", self.theme.title_style()),
                    Span::raw(stats.total_repositories.to_string()),
                ]),
                Line::from(vec![
                    Span::styled("Modules: ", self.theme.title_style()),
                    Span::raw(stats.total_modules.to_string()),
                    Span::raw(" | "),
                    Span::styled("Tasks: ", self.theme.title_style()),
                    Span::raw(stats.total_tasks.to_string()),
                    Span::raw(" | "),
                    Span::styled("Documents: ", self.theme.title_style()),
                    Span::raw(stats.total_documents.to_string()),
                ]),
            ];

            Paragraph::new(stats_content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(self.theme.border_style())
                        .title(Span::styled(
                            "Database Statistics",
                            self.theme.title_style(),
                        )),
                )
                .render(chunks[0], buf);
        }

        // Query section
        let query_lines: Vec<Line> = self.query.lines().map(|line| Line::from(line)).collect();

        let mut query_block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title(Span::styled("Query Editor", self.theme.title_style()));

        if self.query_error.is_some() {
            query_block = query_block.border_style(Style::default().fg(Color::Red));
        }

        Paragraph::new(query_lines)
            .block(query_block)
            .render(chunks[1], buf);

        // Results section
        let mut result_content = vec![];
        if let Some(error) = self.query_error {
            result_content.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::raw(error),
            ]));
        } else if let Some(result) = self.query_result {
            for line in result.lines().take(20) {
                result_content.push(Line::from(line));
            }
            if result.lines().count() > 20 {
                result_content.push(Line::from("... (truncated)"));
            }
        } else {
            result_content.push(Line::from("No results"));
        }

        Paragraph::new(result_content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Query Results", self.theme.title_style())),
            )
            .render(chunks[2], buf);
    }
}

/// Table Browser Widget
pub struct TableBrowserWidget<'a> {
    tables: &'a [String],
    selected_table: usize,
    table_data: Option<&'a Vec<Vec<String>>>,
    theme: &'a Theme,
}

impl<'a> TableBrowserWidget<'a> {
    pub fn new(
        tables: &'a [String],
        selected_table: usize,
        table_data: Option<&'a Vec<Vec<String>>>,
        theme: &'a Theme,
    ) -> Self {
        Self {
            tables,
            selected_table,
            table_data,
            theme,
        }
    }
}

impl<'a> ratatui::widgets::Widget for TableBrowserWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(10)])
            .split(area);

        // Table list
        let table_items: Vec<ListItem> = self
            .tables
            .iter()
            .enumerate()
            .map(|(i, table)| {
                let style = if i == self.selected_table {
                    Style::default()
                        .fg(self.theme.rust_orange)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Span::styled(table, style))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_table));

        List::new(table_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.theme.border_style())
                    .title(Span::styled("Tables", self.theme.title_style())),
            )
            .render(chunks[0], buf);

        // Table data
        if let Some(data) = self.table_data {
            if !data.is_empty() {
                let _headers: Vec<ratatui::widgets::Row> = vec![ratatui::widgets::Row::new(
                    data[0]
                        .iter()
                        .map(|h| Span::styled(h, self.theme.title_style())),
                )];

                let rows: Vec<ratatui::widgets::Row> = data[1..]
                    .iter()
                    .take(20)
                    .map(|row| ratatui::widgets::Row::new(row.iter().map(|cell| Span::raw(cell))))
                    .collect();

                Table::new(rows, [Constraint::Percentage(100)])
                    .header(ratatui::widgets::Row::new(
                        data[0]
                            .iter()
                            .map(|h| Span::styled(h, self.theme.title_style())),
                    ))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(self.theme.border_style())
                            .title(Span::styled(
                                format!("Data: {}", self.tables[self.selected_table]),
                                self.theme.title_style(),
                            )),
                    )
                    .render(chunks[1], buf);
            } else {
                Paragraph::new("No data")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(self.theme.border_style())
                            .title(Span::styled("Table Data", self.theme.title_style())),
                    )
                    .render(chunks[1], buf);
            }
        } else {
            Paragraph::new("Select a table to view data")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(self.theme.border_style())
                        .title(Span::styled("Table Data", self.theme.title_style())),
                )
                .render(chunks[1], buf);
        }
    }
}
