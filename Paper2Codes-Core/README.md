# Paper2Codes Core

The core Rust backend for the Paper2Codes framework, providing paper processing, code generation, and verification capabilities.

## Overview

Paper2Codes Core is a production-ready neuro-symbolic Retrieval-Augmented Generation (RAG) framework that automatically generates executable code and documentation from scientific literature.

The Core backend supports two interfaces:
- **TUI (Terminal User Interface)**: Interactive terminal interface using Ratatui
- **API Server**: HTTP REST API and WebSocket server for integration with the WebUI

For comprehensive integration details, see [INTEGRATION.md](../INTEGRATION.md).

## Features

- **Intelligent Paper Processing**: Parse PDFs and text, extract algorithms, equations, and structure
- **Domain Classification**: Automatic detection of paper domain for optimized code generation
- **Multi-Agent Orchestration**: Parallel execution with resource control and convergence detection
- **Advanced Retrieval**: State-of-the-art embedding models (OpenAI, Voyage AI) with vector search
- **LLM Integration**: Support for OpenAI, Anthropic Claude, xAI Grok via OpenRouter
- **Code Verification**: Comprehensive SACV pipeline with static analysis, dynamic testing, and symbolic reasoning
- **Interactive TUI**: Rich terminal user interface powered by Ratatui
- **API Server**: REST API and WebSocket server for WebUI integration
- **Persistent Storage**: SurrealDB with native vector search and graph capabilities

## Prerequisites

- Rust 1.70+ ([Install Rust](https://www.rust-lang.org/tools/install))
- SurrealDB (optional, for persistent storage) ([Install SurrealDB](https://surrealdb.com/docs/installation))
- API Keys for LLM providers (OpenAI, Anthropic, or OpenRouter)

## Installation

### From Source

```bash
# Build with optimizations
cargo build --release

# Install globally
cargo install --path .

# Run
paper2codes --help
```

### Configuration

1. **Set up configuration**:
   ```bash
   mkdir -p ~/.config/paper2codes
   cp config.example.toml ~/.config/paper2codes/config.toml
   # Edit config.toml with your API keys and settings
   ```

2. **Set environment variables** (recommended for API keys):
   ```bash
   export OPENROUTER_API_KEY="sk-or-v1-..."
   # Or for other providers:
   export OPENAI_API_KEY="sk-..."
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

3. **Start SurrealDB** (persistent storage):
   ```bash
   cd Paper2Codes-Core
   ./scripts/start_surrealdb.sh
   ```

   The helper script boots SurrealDB on `ws://127.0.0.1:8000` with the default namespace `paper2codes` and database `main`.  
   If you prefer to run it manually:

   ```bash
   surreal start --bind 127.0.0.1:8000 --user root --pass root file://./data/surrealdb
   ```

4. **Enable storage & API server in `~/.config/paper2codes/config.toml`**:
   ```toml
   [storage]
   enabled = true
   connection_string = "ws://127.0.0.1:8000"
   namespace = "paper2codes"
   database = "main"
   username = "root"
   password = "root"
   auto_migrate = true

   [api]
   enabled = true
   host = "127.0.0.1"
   port = 8080
   ```

   On startup the API server now verifies the SurrealDB connection, applies any missing schema elements, and validates the existing schema.

5. **Verify the database connection** (optional):
   ```bash
   curl -s http://127.0.0.1:8080/api/settings/database/test | jq
   ```

## Usage

### Command-Line Interface

```bash
# Process a PDF paper
paper2codes process --paper path/to/paper.pdf --output ./generated_code

# Process text format
paper2codes process --paper path/to/paper.txt --output ./generated_code

# With verbose logging
paper2codes process --paper paper.pdf --output ./output --verbose
```

### Interactive TUI

```bash
# Launch interactive interface
paper2codes tui

# With pre-loaded paper
paper2codes tui --paper path/to/paper.pdf
```

### API Server Mode

```bash
# Start API server (runs alongside TUI if enabled)
paper2codes api

# Start API server on specific host and port
paper2codes api --host 0.0.0.0 --port 8080

# Start API server with TUI disabled
paper2codes api --no-tui
```

The API server exposes:
- **REST API**: HTTP endpoints for papers, repositories, tasks, modules
- **WebSocket**: Real-time updates for task progress and status changes
- **Authentication**: JWT-based authentication endpoints

For detailed API specifications and endpoints, see [INTEGRATION.md](../INTEGRATION.md).

### Health Check

```bash
paper2codes health
```

## Project Structure

```
Paper2Codes-Core/
├── src/
│   ├── api/             # API server layer
│   │   ├── handlers/    # REST API handlers
│   │   ├── middleware/  # Auth, CORS, logging, rate limiting
│   │   ├── websocket/   # WebSocket server
│   │   ├── types/       # Request/response DTOs
│   │   └── server.rs    # Server setup
│   ├── agents/          # Multi-agent system
│   ├── config/          # Configuration management
│   ├── coordinator/     # Task orchestration
│   ├── document/        # Document processing
│   ├── domain/          # Domain detection
│   ├── llm/             # LLM integration
│   ├── retrieval/       # RAG and embeddings
│   ├── storage/         # SurrealDB integration
│   ├── verification/    # Code verification
│   ├── ui/              # TUI interface (ratatui)
│   ├── main.rs          # CLI entry point
│   └── lib.rs           # Library entry point
├── examples/            # Example usage
├── scripts/             # Utility scripts
├── tests/               # Unit and integration tests
└── Cargo.toml           # Rust project configuration
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- process --paper examples/sample_paper.txt

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## API Integration

The Core can be used in multiple ways:

1. **As a Library**: Import and use the Core functionality programmatically
2. **TUI Mode**: Interactive terminal interface for direct use
3. **API Server Mode**: HTTP REST API and WebSocket server for WebUI integration

The API server provides:
- REST endpoints for all Core functionality (papers, repositories, tasks, modules)
- WebSocket connections for real-time updates
- JWT-based authentication and authorization
- Rate limiting and request validation

For detailed API specifications, endpoint definitions, and integration guide, see [INTEGRATION.md](../INTEGRATION.md).

## Documentation

- [Main README](../README.md) - Project overview
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture
- [SPECS.md](../SPECS.md) - Technical specifications
- [API Documentation](https://docs.rs/paper2codes) - Rust API docs

## License

MIT License - see the main project LICENSE file.
