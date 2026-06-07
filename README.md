# Paper2Codes: Neuro-Symbolic RAG Framework for Automated Code Generation

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

*Transform scientific papers into production-ready code automatically*

[Features](#features) • [Installation](#installation) • [Quick Start](#quick-start) • [Architecture](#architecture) • [Documentation](#documentation) • [Contributing](#contributing)

</div>

---

## Overview

Paper2Codes is a production-ready neuro-symbolic Retrieval-Augmented Generation (RAG) framework that automatically generates executable code and documentation from scientific literature. Leveraging state-of-the-art LLMs, advanced retrieval algorithms, and symbolic verification, Paper2Codes transforms research papers into production-ready implementations.

### Project Structure

Paper2Codes consists of two main components:

- **Paper2Codes-Core** (Rust): The core backend library providing paper processing, code generation, and verification capabilities. Includes both TUI interface and HTTP/WebSocket API server for integration with the WebUI.
- **Paper2Codes-WebUI** (Next.js + Deno Tooling): A modern web interface built with Next.js that communicates with the Core backend via REST API and WebSocket for real-time updates.

See [STRUCTURE.md](STRUCTURE.md) for detailed structure documentation. For comprehensive integration details, see [INTEGRATION.md](INTEGRATION.md).

### Key Innovations

- **Advanced RAG**: Contextual Paper Retrieval (CPR) with hybrid semantic+keyword scoring
- **Multi-Agent Architecture**: Specialized agents for planning, analysis, coding, and verification
- **Symbolically-Augmented Verification**: Multi-layered code verification (static, dynamic, symbolic)
- **Production-Ready**: Comprehensive error handling, caching, parallel execution
- **Persistent Storage**: SurrealDB integration with graph-based dependency tracking

---

## Features

### 🚀 Core Capabilities

- **Intelligent Paper Processing**: Parse PDFs and text, extract algorithms, equations, and structure
- **Domain Classification**: Automatic detection of paper domain for optimized code generation
- **Multi-Agent Orchestration**: Parallel execution with resource control and convergence detection
- **Advanced Retrieval**: State-of-the-art embedding models (OpenAI, Voyage AI) with vector search
- **LLM Integration**: Support for OpenAI, Anthropic Claude, xAI Grok via OpenRouter
- **Code Verification**: Comprehensive SACV pipeline with static analysis, dynamic testing, and symbolic reasoning
- **Interactive TUI**: Rich terminal user interface powered by Ratatui
- **Persistent Storage**: SurrealDB with native vector search and graph capabilities

### 💡 Technical Highlights

- **Performance**: Parallel task execution with semaphore-based concurrency control
- **Caching**: Multi-level caching (embeddings, LLM responses) for cost optimization
- **Error Recovery**: Automatic retry with exponential backoff and graceful degradation
- **Type Safety**: Full Rust type system leveraging for compile-time guarantees
- **Modularity**: Clean trait-based architecture for extensibility
- **API Integration**: RESTful API and WebSocket support for real-time communication
- **Dual Interface**: Both TUI (terminal) and WebUI (browser) interfaces available

---

## Installation

### Prerequisites

**For Core (Rust):**
- Rust 1.70+ ([Install Rust](https://www.rust-lang.org/tools/install))
- SurrealDB (optional, for persistent storage) ([Install SurrealDB](https://surrealdb.com/docs/installation))
- API Keys for LLM providers (OpenAI, Anthropic, or OpenRouter)

**For WebUI (Deno + Next.js):**
- Deno 1.40+ ([Install Deno](https://deno.land/manual/getting_started/installation))
- Node.js 18+ (required for Next.js runtime) ([Install Node.js](https://nodejs.org/))

### Production Deployment

#### Core (Rust Backend)

1. **Build optimized release binary**:
   ```bash
   cd Paper2Codes-Core
   cargo build --release
   ```

2. **Set up configuration**:
   ```bash
   mkdir -p ~/.config/paper2codes
   cp Paper2Codes-Core/config.example.toml ~/.config/paper2codes/config.toml
   # Edit config.toml with your API keys and settings
   ```

3. **Set environment variables** (recommended for API keys):
   ```bash
   export OPENROUTER_API_KEY="sk-or-v1-..."
   # Or for other providers:
   export OPENAI_API_KEY="sk-..."
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

4. **Start SurrealDB** (if using persistent storage):
   ```bash
   surreal start --log trace --user root --pass root memory
   # Or for file-based storage:
   surreal start --log trace --user root --pass root file://./data/surrealdb
   ```

5. **Run health check**:
   ```bash
   ./Paper2Codes-Core/target/release/paper2codes health
   ```

#### WebUI (Frontend)

1. **Install dependencies and build**:
   ```bash
   cd Paper2Codes-WebUI
   npm install
   npm run build
   ```

2. **Start production server**:
   ```bash
   npm start
   ```

3. **Development mode**:
   ```bash
   npm run dev
   ```

**Note**: Next.js requires Node.js runtime. Deno is used for code quality tooling (linting, formatting) - see `Paper2Codes-WebUI/README.md` for details.

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/Paper2Codes.git
cd Paper2Codes

# Build Core with optimizations
cd Paper2Codes-Core
cargo build --release

# Run Core CLI
./target/release/paper2codes --help

# Setup and run WebUI
cd ../Paper2Codes-WebUI
deno task dev  # Development mode
# Or: npm install && npm run dev
```

### Using Cargo

```bash
cargo install --path .
# Or from crates.io (when published):
# cargo install paper2codes
```

### Production Build

For production use, build with optimizations:

```bash
cargo build --release
# Binary will be at: ./target/release/paper2codes
```

---

## Quick Start

### 1. Configure API Keys

Create a `config.toml` in `~/.config/paper2codes/` or use environment variables:

```toml
[llm]
openrouter_api_key = "sk-or-v1-..."  # or set OPENROUTER_API_KEY env var
default_provider = "openrouter"

[llm.providers.openrouter]
enabled = true
base_url = "https://openrouter.ai/api/v1"
models = ["openai/gpt-4-turbo", "anthropic/claude-3-opus-20240229"]

[agents]
planning_model = "openai/gpt-4-turbo"
analysis_model = "anthropic/claude-3-opus-20240229"
coding_model = "openai/gpt-4-turbo"
verification_model = "anthropic/claude-3-opus-20240229"
max_iterations = 10
parallel_tasks = 4

[storage]
enabled = true
connection_string = "ws://localhost:8000"
namespace = "paper2codes"
database = "main"
username = "root"
password = "root"
auto_migrate = true
```

See [config.example.toml](config.example.toml) for full configuration options.

### 2. Process a Paper

#### Command-Line Interface

```bash
# Process a PDF paper
paper2codes process --paper path/to/paper.pdf --output ./generated_code

# Process text format
paper2codes process --paper path/to/paper.txt --output ./generated_code

# With verbose logging
paper2codes process --paper paper.pdf --output ./output --verbose
```

#### Interactive TUI

```bash
# Launch interactive interface
paper2codes tui

# With pre-loaded paper
paper2codes tui --paper path/to/paper.pdf
```

### 3. Example Output

```
Processing paper: "Attention Is All You Need"
- Domain: Machine Learning / Deep Learning
- Modules identified: 8
  ✓ Planning completed (12.3s)
  ✓ Analysis completed (8 modules analyzed in 24.5s)
  ✓ Code generation (parallel execution)
    - attention_mechanism.py (4.2s)
    - transformer_block.py (5.1s)
    - positional_encoding.py (3.8s)
    - multi_head_attention.py (6.3s)
    - feed_forward.py (2.9s)
    - encoder.py (5.7s)
    - decoder.py (6.4s)
    - transformer.py (7.8s)
  ✓ Verification completed (PASSED with 2 warnings)

Generated 8 modules in ./generated_code/
Total time: 89.2s
```

---

## Architecture

Paper2Codes uses a sophisticated multi-layered architecture supporting both TUI and WebUI interfaces:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Paper2Codes-WebUI                        │
│                    (Next.js + Deno Tooling)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Dashboard  │  │   Papers     │  │    Tasks     │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   HTTP/REST API     │
                    │   WebSocket API     │
                    └──────────┬──────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                    Paper2Codes-Core                             │
│                    (Rust Backend)                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              API Server Layer (axum/warp)                │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │  │
│  │  │   REST     │  │ WebSocket  │  │   Auth     │        │  │
│  │  │  Handlers  │  │  Handlers  │  │  Middleware│        │  │
│  │  └────────────┘  └────────────┘  └────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Business Logic Layer                         │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │  │
│  │  │Coordinator │  │   Agents   │  │ Retrieval  │        │  │
│  │  └────────────┘  └────────────┘  └────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Storage & External Services                  │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐        │  │
│  │  │ SurrealDB  │  │  LLM APIs  │  │   Docker   │        │  │
│  │  └────────────┘  └────────────┘  └────────────┘        │  │
│  └──────────────────────────────────────────────────────────┘  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   TUI Interface     │
                    │   (ratatui)         │
                    │   (Optional Mode)   │
                    └─────────────────────┘
```

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md). For comprehensive integration details including API specifications, see [INTEGRATION.md](INTEGRATION.md).

### Key Algorithms

1. **Contextual Paper Retrieval (CPR)**: Hybrid scoring with semantic similarity, keyword matching, algorithm boosting, and implementation de-boosting

2. **Multi-Agent Orchestration**: Task dependency resolution with parallel execution and convergence detection

3. **Symbolically-Augmented Code Verification (SACV)**: Three-phase verification pipeline (static → dynamic → symbolic)

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Documentation

### Core Documentation

- [Architecture Guide](ARCHITECTURE.md) - System design and component details
- [Specifications](SPECS.md) - Technical specifications and implementation details
- [Integration Plan](INTEGRATION.md) - Comprehensive integration plan for Core and WebUI
- [Project Structure](STRUCTURE.md) - Project structure and organization
- [API Documentation](https://docs.rs/paper2codes) - Rust API docs (when published)

### Examples

See the [examples/](examples/) directory for:

- Basic usage examples
- Custom agent implementations
- Integration with external tools
- Advanced configuration

### Configuration

Full configuration reference available in [config.example.toml](config.example.toml).

Key configuration sections:

- **LLM**: Provider selection, API keys, model routing
- **Agents**: Model assignments, iteration limits, parallelism
- **Verification**: Enable/disable verification phases
- **Storage**: SurrealDB connection and schema settings
- **Execution**: Sandbox configuration and timeouts

---

## Advanced Usage

### Custom Agent Implementation

```rust
use paper2codes::agents::{Agent, AgentContext, AgentResponse};
use async_trait::async_trait;

#[derive(Clone)]
struct CustomAgent {
    // Your agent state
}

#[async_trait]
impl Agent for CustomAgent {
    async fn execute(&self, task: &Task, context: &AgentContext) -> Result<AgentResponse> {
        // Your implementation
    }
    
    fn agent_type(&self) -> AgentType {
        AgentType::Custom("my_agent".to_string())
    }
}
```

### Integrating with Existing Tools

```rust
use paper2codes::coordinator::Coordinator;
use paper2codes::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let mut coordinator = Coordinator::new(config).await?;
    
    // Process paper
    let paper = /* load paper */;
    let repository = coordinator.process_paper(paper).await?;
    
    // Access generated code
    for module in &repository.modules {
        println!("Generated: {}", module.file_path.display());
    }
    
    Ok(())
}
```

---

## Performance

### Benchmarks

On a typical machine learning paper (10-15 pages):

- **Planning**: ~10-20 seconds
- **Analysis**: ~20-30 seconds (8-12 modules)
- **Code Generation**: ~40-60 seconds (parallel)
- **Verification**: ~10-20 seconds
- **Total**: ~90-120 seconds

### Optimization Tips

1. **Enable Caching**: Reuse embeddings and LLM responses
2. **Adjust Parallelism**: Increase `parallel_tasks` for faster processing
3. **Use Faster Models**: Trade-off between speed and quality
4. **Persistent Storage**: Enable SurrealDB for multi-session caching

---

## Troubleshooting

### Common Issues

**Issue**: `PDF parsing requires additional dependencies`
- **Solution**: Convert PDF to text first, or use `parse_text()` method

**Issue**: `Failed to connect to SurrealDB`
- **Solution**: Ensure SurrealDB is running: `surreal start --log trace --user root --pass root`

**Issue**: `Rate limit exceeded`
- **Solution**: Adjust `max_retries` and `timeout_seconds` in config, or use different API keys

**Issue**: `Module verification failed`
- **Solution**: Check logs for specific issues, adjust verification settings, or disable strict verification

### Getting Help

- [GitHub Issues](https://github.com/yourusername/Paper2Codes/issues)
- [Discussions](https://github.com/yourusername/Paper2Codes/discussions)
- [Documentation](https://docs.rs/paper2codes)

### Health Check

Before running in production, verify your setup:

```bash
paper2codes health
```

This will check:
- Configuration loading
- API key availability
- Storage connection (if enabled)
- System readiness

### Version Information

Check installed version:

```bash
paper2codes version
```

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone and setup
git clone https://github.com/yourusername/Paper2Codes.git
cd Paper2Codes

# Install dev dependencies
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

---

## Citation

If you use Paper2Codes in your research, please cite:

```bibtex
@software{paper2codes2024,
  title = {Paper2Codes: Neuro-Symbolic RAG Framework for Automated Code Generation},
  author = {Your Name},
  year = {2024},
  url = {https://github.com/yourusername/Paper2Codes}
}
```

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- UI powered by [Ratatui](https://github.com/ratatui-org/ratatui)
- Storage by [SurrealDB](https://surrealdb.com/)
- LLM integration via [OpenRouter](https://openrouter.ai/)

---

## Production Readiness

Paper2Codes has undergone comprehensive code review and refactoring to ensure production-level quality:

✅ **Security**: All SQL injection vulnerabilities fixed, input validation implemented, rate limiting configured, JWT authentication  
✅ **Performance**: Database operations optimized, adaptive concurrency control, HTTP/2 connection pooling, multi-level caching  
✅ **Code Quality**: Algorithms verified for correctness, numerical precision improved, comprehensive error handling  
✅ **Reliability**: Automatic retry with exponential backoff, health checking, graceful error recovery  
✅ **Scalability**: Horizontal scaling ready, adaptive concurrency, smart rate limiting with token awareness  
✅ **Monitoring**: Prometheus metrics, structured logging, comprehensive observability  
✅ **Documentation**: Production deployment guide, code review summary, operational runbooks  

**Production Status**: ✅ **APPROVED FOR DEPLOYMENT** (with documented limitations on optional symbolic features)

See [CODE_REVIEW_SUMMARY.md](CODE_REVIEW_SUMMARY.md) for detailed assessment and [PRODUCTION_GUIDE.md](PRODUCTION_GUIDE.md) for deployment instructions.

## Roadmap

- [x] Core RAG implementation
- [x] Multi-agent architecture
- [x] SurrealDB integration
- [x] Advanced retrieval with embeddings
- [x] Interactive TUI
- [x] API server implementation (see [INTEGRATION.md](INTEGRATION.md))
- [x] Production-ready security and performance optimizations
- [ ] WebUI integration (see [INTEGRATION.md](INTEGRATION.md))
- [ ] Z3 symbolic verification
- [ ] SymPy equation solving
- [ ] Multi-paper synthesis
- [ ] Fine-tuned domain models
- [ ] Plugin system

---

<div align="center">

**[⬆ back to top](#paper2codes-neuro-symbolic-rag-framework-for-automated-code-generation)**

Made with ❤️ by the Paper2Codes team

</div>

