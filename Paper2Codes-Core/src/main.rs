// Import from library
use paper2codes::{
    config::Config, coordinator::Coordinator, document::DocumentProcessor, error::Result, ui,
};

use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "paper2codes")]
#[command(version = paper2codes::VERSION)]
#[command(about = "A neuro-symbolic RAG framework for automated code generation from scientific literature", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a research paper and generate code
    Process {
        /// Path to the paper (PDF or text file)
        #[arg(short, long)]
        paper: String,

        /// Output directory for generated code
        #[arg(short, long, default_value = "./output")]
        output: String,

        /// Domain skill id to specialise generation (e.g. computational-physics)
        #[arg(long)]
        skill: Option<String>,

        /// Target language for generation (defaults to the skill's default)
        #[arg(long)]
        language: Option<String>,
    },
    /// Run the interactive TUI
    Tui {
        /// Path to the paper (PDF or text file)
        #[arg(short, long)]
        paper: Option<String>,
    },
    /// Show configuration
    Config {
        /// Show default configuration
        #[arg(long)]
        default: bool,
    },
    /// Show version information
    Version,
    /// Check system health and configuration
    Health,
    /// Start the API server
    Api {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Run the reproducibility benchmark harness over a manifest
    Bench {
        /// Path to the benchmark manifest (JSON)
        #[arg(short, long)]
        manifest: String,

        /// Optional path to write the JSON report
        #[arg(short, long)]
        output: Option<String>,

        /// Run fully offline with a deterministic stub generator (no API keys).
        /// Validates the harness, dataset, and scoring before keys are configured.
        #[arg(long)]
        offline: bool,

        /// Target language for offline generation
        #[arg(long, default_value = "python")]
        language: String,
    },
}

/// Benchmark generator that runs the full paper→repository pipeline per case.
struct CliRepoGenerator;

#[async_trait::async_trait]
impl paper2codes::benchmark::RepoGenerator for CliRepoGenerator {
    async fn generate(
        &self,
        case: &paper2codes::benchmark::BenchmarkCase,
    ) -> Result<paper2codes::benchmark::GeneratedRepo> {
        use paper2codes::error::Paper2CodesError;

        let config = Config::load()?;
        let mut coordinator = Coordinator::new(config).await?;
        let processor = DocumentProcessor::new();

        let path = &case.paper_path;
        let mut paper = if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            processor.parse_pdf(path).await?
        } else {
            let content = std::fs::read_to_string(path).map_err(Paper2CodesError::Io)?;
            processor.parse_text(&content, None).await?
        };
        processor.segment_paper(&mut paper)?;

        let repository = coordinator.process_paper(paper).await?;
        Ok(paper2codes::benchmark::GeneratedRepo::from_repository(
            &repository,
        ))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new(Level::DEBUG.to_string())
    } else {
        EnvFilter::new(Level::INFO.to_string())
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting Paper2Codes engine");

    // Load configuration
    let _config = Config::load()?;
    info!("Configuration loaded");

    match cli.command {
        Commands::Process {
            paper,
            output,
            skill,
            language,
        } => {
            info!("Processing paper: {}", paper);
            info!("Output directory: {}", output);

            // Validate paper path exists
            let paper_path = std::path::PathBuf::from(&paper);
            if !paper_path.exists() {
                return Err(paper2codes::error::Paper2CodesError::Config(
                    paper2codes::error::ConfigError::Invalid(format!(
                        "Paper file not found: {}",
                        paper_path.display()
                    )),
                ));
            }

            // Validate output directory can be created
            let output_path = std::path::PathBuf::from(&output);
            if let Some(parent) = output_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        paper2codes::error::Paper2CodesError::Config(
                            paper2codes::error::ConfigError::Invalid(format!(
                                "Failed to create output directory {}: {}",
                                parent.display(),
                                e
                            )),
                        )
                    })?;
                }
            }

            // Load configuration with better error context
            let config = Config::load().map_err(|e| {
                paper2codes::error::Paper2CodesError::Config(
                    paper2codes::error::ConfigError::Invalid(
                        format!("Failed to load configuration: {}. Hint: Check ~/.config/paper2codes/config.toml or environment variables.", e)
                    )
                )
            })?;

            // Initialize coordinator with better error context
            let mut coordinator = Coordinator::new(config).await.map_err(|e| {
                paper2codes::error::Paper2CodesError::Coordinator(
                    format!("Failed to initialize coordinator: {}. Hint: Check API keys and storage connection.", e)
                )
            })?;

            // Load and parse paper with better error handling
            let document_processor = DocumentProcessor::new();
            let mut paper = if paper_path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                document_processor
                    .parse_pdf(&paper_path)
                    .await
                    .map_err(|e| {
                        paper2codes::error::Paper2CodesError::Document(
                            paper2codes::error::DocumentError::ParseFailed(format!(
                                "Failed to parse PDF {}: {}. Hint: Ensure the file is a valid PDF.",
                                paper_path.display(),
                                e
                            )),
                        )
                    })?
            } else {
                let content = std::fs::read_to_string(&paper_path).map_err(|e| {
                    paper2codes::error::Paper2CodesError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Failed to read paper file {}: {}", paper_path.display(), e),
                    ))
                })?;
                document_processor.parse_text(&content, None).await.map_err(|e| {
                    paper2codes::error::Paper2CodesError::Document(
                        paper2codes::error::DocumentError::ParseFailed(
                            format!("Failed to parse text content: {}. Hint: Ensure the file contains valid text.", e)
                        )
                    )
                })?
            };

            // Segment paper with better error context
            document_processor.segment_paper(&mut paper).map_err(|e| {
                paper2codes::error::Paper2CodesError::Document(
                    paper2codes::error::DocumentError::ParseFailed(format!(
                        "Failed to segment paper: {}. Hint: The paper content may be malformed.",
                        e
                    )),
                )
            })?;

            // Apply the domain-skill generation profile, if requested.
            if let Some(skill_id) = &skill {
                let mut registry = paper2codes::skills::SkillRegistry::with_builtins();
                if let Ok(dir) = paper2codes::skills::SkillRegistry::user_skills_dir() {
                    let _ = registry.load_user_dir(&dir);
                }
                match registry.get(skill_id) {
                    Some(skill) => {
                        let lang = language.clone().unwrap_or_else(|| skill.default_language().to_string());
                        info!("Using domain skill '{}' ({}) for generation", skill.name, lang);
                        println!("Domain skill: {} ({})", skill.name, lang);
                        coordinator.set_generation_profile(Some(skill.clone()), Some(lang));
                    }
                    None => {
                        return Err(paper2codes::error::Paper2CodesError::Validation(format!(
                            "Unknown skill '{}'. Run with a valid skill id (see /api/skills).",
                            skill_id
                        )));
                    }
                }
            }

            // Process paper with better error context
            coordinator.set_output_path(output_path.clone());
            let repository = coordinator.process_paper(paper).await.map_err(|e| {
                paper2codes::error::Paper2CodesError::Coordinator(
                    format!("Failed to process paper: {}. Hint: Check LLM API keys and network connectivity.", e)
                )
            })?;

            info!(
                "Processing completed. Generated {} modules",
                repository.modules.len()
            );
            println!(
                "Processing completed. Generated {} modules in {}",
                repository.modules.len(),
                repository.root_path.display()
            );

            Ok(())
        }
        Commands::Tui { paper } => {
            info!("Starting TUI");
            let (mut app, _event_tx) = if let Some(paper_path) = paper {
                info!("Paper provided: {}", paper_path);
                ui::App::new_with_paper(std::path::PathBuf::from(paper_path))
            } else {
                ui::App::new()
            };

            // Run the TUI with better error handling
            app.run().await.map_err(|e| {
                paper2codes::error::Paper2CodesError::Coordinator(format!(
                    "TUI error: {}. Hint: Check terminal compatibility and dependencies.",
                    e
                ))
            })?;
            Ok(())
        }
        Commands::Config { default } => {
            if default {
                let default_config = Config::default();
                let toml = toml::to_string_pretty(&default_config)?;
                println!("{}", toml);
            } else {
                let config = Config::load()?;
                let toml = toml::to_string_pretty(&config)?;
                println!("{}", toml);
            }
            Ok(())
        }
        Commands::Version => {
            println!("{}", paper2codes::info());
            Ok(())
        }
        Commands::Health => {
            info!("Running health check");
            let mut issues: Vec<String> = Vec::new();
            let mut warnings: Vec<String> = Vec::new();

            // Check configuration
            match Config::load() {
                Ok(config) => {
                    info!("✓ Configuration loaded successfully");

                    // Check API keys with better error handling
                    let has_openrouter_key = config
                        .llm
                        .providers
                        .get("openrouter")
                        .and_then(|p| p.api_key.as_ref())
                        .map(|k| !k.is_empty())
                        .unwrap_or(false)
                        || std::env::var("OPENROUTER_API_KEY").is_ok();

                    let has_openai_key = config
                        .llm
                        .providers
                        .get("openai")
                        .and_then(|p| p.api_key.as_ref())
                        .map(|k| !k.is_empty())
                        .unwrap_or(false)
                        || std::env::var("OPENAI_API_KEY").is_ok();

                    let has_anthropic_key = config
                        .llm
                        .providers
                        .get("anthropic")
                        .and_then(|p| p.api_key.as_ref())
                        .map(|k| !k.is_empty())
                        .unwrap_or(false)
                        || std::env::var("ANTHROPIC_API_KEY").is_ok();

                    if !has_openrouter_key && !has_openai_key && !has_anthropic_key {
                        issues.push("No LLM API keys found. Set OPENROUTER_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY environment variable, or configure in config.toml".to_string());
                    } else {
                        info!("✓ At least one API key configured");
                        if has_openrouter_key {
                            info!("  - OpenRouter API key: ✓");
                        }
                        if has_openai_key {
                            info!("  - OpenAI API key: ✓");
                        }
                        if has_anthropic_key {
                            info!("  - Anthropic API key: ✓");
                        }
                    }
                }
                Err(e) => {
                    issues.push(format!("Failed to load configuration: {}", e));
                }
            }

            // Check storage connection if enabled
            match Config::load() {
                Ok(config) => {
                    if config.storage.enabled {
                        info!("Checking storage connection...");
                        // Initialize storage manager
                        let _storage_manager = paper2codes::storage::StorageManager::new();
                        // Storage health check would go here
                        // For now, just check if configuration is valid
                        info!("✓ Storage configuration present");
                        info!("  Connection string: {}", config.storage.connection_string);
                    } else {
                        info!("Storage is disabled in configuration");
                    }
                }
                Err(_) => {
                    // Already handled in config check above
                }
            }

            // Check if backend API is running (if trying to start API)
            if let Ok(_) = std::env::var("BACKEND_API_RUNNING") {
                let backend_url = format!(
                    "http://{}:{}/api/health",
                    std::env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                    std::env::var("BACKEND_PORT").unwrap_or_else(|_| "8080".to_string())
                );
                info!("Checking backend API health...");
                match reqwest::get(&backend_url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            info!("✓ Backend API is running and healthy");
                        } else {
                            warnings.push(format!(
                                "Backend API returned status: {}",
                                response.status()
                            ));
                        }
                    }
                    Err(_) => {
                        warnings.push(
                            "Backend API is not reachable. It may not be running.".to_string(),
                        );
                    }
                }
            }

            // Print results
            if issues.is_empty() && warnings.is_empty() {
                println!("✓ Health check passed - system is ready");
                Ok(())
            } else {
                if !warnings.is_empty() {
                    println!("\n⚠ Warnings:");
                    for warning in &warnings {
                        println!("  - {}", warning);
                    }
                }
                if !issues.is_empty() {
                    println!("\n✗ Issues found:");
                    for issue in &issues {
                        println!("  - {}", issue);
                    }
                    Err(paper2codes::error::Paper2CodesError::Config(
                        paper2codes::error::ConfigError::Invalid(format!(
                            "Health check failed with {} issue(s)",
                            issues.len()
                        )),
                    ))
                } else {
                    println!("\n✓ Health check completed with warnings");
                    Ok(())
                }
            }
        }
        Commands::Api { host, port } => {
            #[cfg(feature = "api")]
            {
                info!("Starting API server on {}:{}", host, port);

                // Load configuration
                let mut config = Config::load()?;

                // Override API config from command line
                config.api.enabled = true;
                config.api.host = host.clone();
                config.api.port = port.clone();

                // Create and start API server
                let server = paper2codes::api::ApiServer::new(config).await?;
                let addr: std::net::SocketAddr =
                    format!("{}:{}", host, port).parse().map_err(|e| {
                        paper2codes::error::Paper2CodesError::Config(
                            paper2codes::error::ConfigError::Invalid(format!(
                                "Invalid address {}:{}: {}",
                                host, port, e
                            )),
                        )
                    })?;

                server.serve(addr).await?;
                Ok(())
            }
            #[cfg(not(feature = "api"))]
            {
                eprintln!(
                    "API server is not available. Please rebuild with 'api' feature enabled."
                );
                Err(paper2codes::error::Paper2CodesError::Config(
                    paper2codes::error::ConfigError::Invalid("API feature not enabled".to_string()),
                ))
            }
        }
        Commands::Bench {
            manifest,
            output,
            offline,
            language,
        } => {
            let manifest_data = paper2codes::benchmark::BenchmarkManifest::load(&manifest)?;
            println!(
                "Running benchmark{}{} over {} case(s)...",
                manifest_data
                    .name
                    .as_ref()
                    .map(|n| format!(" '{}'", n))
                    .unwrap_or_default(),
                if offline { " [offline]" } else { "" },
                manifest_data.cases.len()
            );

            // Offline mode uses a deterministic stub generator so the harness can
            // be validated without API keys. Otherwise the real coordinator runs.
            // Reference-based scoring is automatic; reference-free rubric grading
            // is available via the RubricGrader trait.
            let report = if offline {
                let generator = paper2codes::benchmark::OfflineStubGenerator::new(language);
                paper2codes::benchmark::run_benchmark(&manifest_data, &generator, None).await
            } else {
                let generator = CliRepoGenerator;
                paper2codes::benchmark::run_benchmark(&manifest_data, &generator, None).await
            };

            println!("\n{}", report.to_markdown());

            if let Some(out) = output {
                std::fs::write(&out, report.to_json()?)
                    .map_err(paper2codes::error::Paper2CodesError::Io)?;
                println!("Report written to {}", out);
            }

            Ok(())
        }
    }
}
