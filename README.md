<div align="center">

<h1>Paper2Codes</h1>

### From paper to production — automatically.

**by [Dirac Technologies](#about-dirac-technologies)**

Paper2Codes turns scientific papers into verified, runnable code. Upload a PDF,
get a working repository — planned, written, and checked by a team of
specialized AI agents grounded in neuro-symbolic retrieval.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Frontend](https://img.shields.io/badge/web-Next.js-black.svg)](https://nextjs.org)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Status](https://img.shields.io/badge/status-production%20ready-blue.svg)]()

[**Why Paper2Codes**](#why-paper2codes) ·
[**How it works**](#how-it-works) ·
[**Quickstart**](#quickstart) ·
[**Architecture**](#architecture) ·
[**Docs**](#documentation)

</div>

---

## The problem

Reproducing a research paper is slow, manual, and error-prone. A single method
section can hide days of work: deciphering notation, reconstructing algorithms,
wiring up modules, and debugging the gap between math and code. Most papers are
never reproduced at all.

## The product

**Paper2Codes is the reproduction layer for research.** Point it at a paper and
it delivers a structured, verified codebase — not a single hallucinated file,
but a planned repository with modules, dependencies, and tests.

It is built on a **neuro-symbolic Retrieval-Augmented Generation (RAG)** engine:
large language models for reasoning and generation, symbolic methods for
verification, and a purpose-built retrieval layer that keeps generation grounded
in the source paper.

```
   paper.pdf  ──▶  Paper2Codes  ──▶  ./generated_code/
                                      ├── attention_mechanism.py   ✓ verified
                                      ├── transformer_block.py     ✓ verified
                                      ├── positional_encoding.py   ✓ verified
                                      └── ...                      8 modules · 89s
```

---

## Why Paper2Codes

| | |
|---|---|
| 🧠 **Grounded, not guessed** | Contextual Paper Retrieval (CPR) keeps every generation anchored to the source text with hybrid semantic + keyword scoring. |
| 🤝 **A team, not a prompt** | Specialized agents for planning, analysis, coding, and verification run in parallel with dependency-aware orchestration. |
| ✅ **Verified by construction** | A three-phase SACV pipeline checks output statically, dynamically, and symbolically — so "generated" means "checked." |
| 🔌 **Bring your own model** | First-class support for OpenAI, Anthropic, OpenRouter, and xAI, switchable live from the web UI with no restart. |
| ⚡ **Fast and frugal** | Parallel execution, adaptive concurrency, HTTP/2 pooling, and multi-level caching of embeddings and LLM responses. |
| 🦀 **Built to last** | A Rust core for compile-time guarantees and predictable performance, with a modern Next.js control plane. |

---

## How it works

Paper2Codes ships as two cooperating components:

- **Paper2Codes-Core** *(Rust)* — the engine. Paper parsing, retrieval, the
  multi-agent coordinator, verification, persistent storage, and both a rich
  terminal UI and an HTTP/WebSocket API.
- **Paper2Codes-WebUI** *(Next.js)* — the control plane. A real-time dashboard
  for uploading papers, watching agents work, browsing generated code, and
  managing provider keys and settings.

```
              Upload ──▶ Plan ──▶ Analyze ──▶ Generate ──▶ Verify ──▶ Repository
                          │         │            │            │
                       planning  analysis      coding     verification
                        agent      agent        agent        agent
                          └──────────┴─────┬──────┴────────────┘
                                  Coordinator (parallel, dependency-aware)
                                           │
                              Neuro-symbolic RAG + SurrealDB
```

---

## Features

### Engine
- **Intelligent paper processing** — parse PDFs and text; extract algorithms, equations, and structure.
- **Automatic domain classification** — detect the paper's field to optimize generation.
- **Multi-agent orchestration** — parallel execution with semaphore-based concurrency, dependency resolution, and convergence detection.
- **Advanced retrieval** — embedding-backed vector search (OpenAI, Voyage AI) with hybrid CPR scoring.
- **SACV verification** — static analysis, dynamic testing, and symbolic reasoning in one pipeline.
- **Persistent storage** — SurrealDB with native vector search and graph-based dependency tracking.

### Platform
- **Live LLM key management** — set, test, rotate, and remove provider keys per provider from the web UI; changes take effect immediately and are stored server-side (never exposed in full). See [LLM_API_KEY.md](LLM_API_KEY.md).
- **Dual interface** — a browser control plane *and* a fully interactive terminal UI (Ratatui).
- **Real-time updates** — REST + WebSocket streaming of task progress and logs.
- **Production hardening** — JWT auth, rate limiting, input validation, structured logging, and Prometheus metrics.
- **Resilience** — automatic retry with exponential backoff and graceful degradation.

---

## Quickstart

### Prerequisites
- **Rust** 1.70+ — [install](https://www.rust-lang.org/tools/install)
- **Node.js** 18+ (for the web UI) — [install](https://nodejs.org/)
- **SurrealDB** (optional, for persistence) — [install](https://surrealdb.com/docs/installation)
- An API key for at least one LLM provider (OpenAI, Anthropic, OpenRouter, or xAI)

### 1. Run the engine

```bash
cd Paper2Codes-Core
cargo build --release

# Configure (API keys can also be added later from the web UI)
mkdir -p ~/.config/paper2codes
cp config.example.toml ~/.config/paper2codes/config.toml

# Or provide keys via environment variables
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Verify your setup
./target/release/paper2codes health
```

### 2. Launch the control plane

```bash
cd Paper2Codes-WebUI
npm install
npm run dev        # http://localhost:3000
```

Open the web UI, go to **Settings → LLM Configuration**, and paste in your
provider keys — they're validated live and applied without a restart.

### 3. Generate code from a paper

**From the CLI:**

```bash
paper2codes process --paper path/to/paper.pdf --output ./generated_code
```

**From the terminal UI:**

```bash
paper2codes tui --paper path/to/paper.pdf
```

**Example run:**

```
Processing paper: "Attention Is All You Need"
- Domain: Machine Learning / Deep Learning
- Modules identified: 8
  ✓ Planning completed (12.3s)
  ✓ Analysis completed (8 modules in 24.5s)
  ✓ Code generation (parallel)
  ✓ Verification PASSED (2 warnings)

Generated 8 modules in ./generated_code/  ·  Total time: 89.2s
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       Paper2Codes-WebUI                          │
│                          (Next.js)                               │
│   ┌────────────┐   ┌────────────┐   ┌────────────┐   ┌────────┐  │
│   │ Dashboard  │   │   Papers   │   │   Tasks    │   │Settings│  │
│   └────────────┘   └────────────┘   └────────────┘   └────────┘  │
└──────────────────────────────┬──────────────────────────────────┘
                               │  REST + WebSocket
┌──────────────────────────────▼──────────────────────────────────┐
│                       Paper2Codes-Core (Rust)                    │
│   API layer        REST · WebSocket · Auth · Rate limiting       │
│   Business logic   Coordinator · Agents · Retrieval · Verify     │
│   Foundations      SurrealDB · LLM providers · Sandbox (Docker)  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                       ┌───────▼────────┐
                       │  Terminal UI   │
                       │   (Ratatui)    │
                       └────────────────┘
```

### Core algorithms

1. **Contextual Paper Retrieval (CPR)** — hybrid retrieval fusing dense embeddings and Okapi **BM25** via **Reciprocal Rank Fusion**, with exact structural-reference boosting ("Algorithm 1", "Equation (5)"), MMR diversity, and implementation-state de-boosting. See [STRATEGY.md](STRATEGY.md) for the retrieval roadmap.
2. **Multi-Agent Orchestration** — dependency-resolved task graph with parallel execution and convergence detection.
3. **Symbolically-Augmented Code Verification (SACV)** — a three-phase pipeline: static → dynamic → symbolic.

Deep dives live in [ARCHITECTURE.md](ARCHITECTURE.md) and [SPECS.md](SPECS.md).

---

## Build for production

```bash
# Engine (optimized binary at ./target/release/paper2codes)
cd Paper2Codes-Core && cargo build --release

# Control plane
cd Paper2Codes-WebUI && npm install && npm run build && npm start
```

Full deployment instructions — Docker Compose, Kubernetes, and monitoring — are
in [DEPLOYMENT.md](DEPLOYMENT.md).

---

## Use it as a library

```rust
use paper2codes::coordinator::Coordinator;
use paper2codes::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut coordinator = Coordinator::new(config).await?;

    let repository = coordinator.process_paper(paper).await?;
    for module in &repository.modules {
        println!("Generated: {}", module.file_path.display());
    }
    Ok(())
}
```

---

## Performance

Typical machine-learning paper (10–15 pages):

| Stage | Time |
|---|---|
| Planning | ~10–20s |
| Analysis (8–12 modules) | ~20–30s |
| Code generation (parallel) | ~40–60s |
| Verification | ~10–20s |
| **Total** | **~90–120s** |

**Tuning:** enable caching, raise `parallel_tasks`, pick faster models for
non-critical stages, and turn on SurrealDB for cross-session reuse.

---

## Documentation

| Guide | What's inside |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design and component internals |
| [SPECS.md](SPECS.md) | Technical specifications |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Production deployment (Docker, K8s, monitoring) |
| [LLM_API_KEY.md](LLM_API_KEY.md) | LLM provider & API key management |
| [Paper2Codes-Core/config.example.toml](Paper2Codes-Core/config.example.toml) | Full configuration reference |
| [TODO.md](TODO.md) | Roadmap and open work |

---

## Production readiness

Paper2Codes has been through comprehensive review and hardening:

- ✅ **Security** — input validation, rate limiting, JWT auth, `0600` secret storage
- ✅ **Performance** — adaptive concurrency, HTTP/2 pooling, multi-level caching
- ✅ **Reliability** — retry with backoff, health checks, graceful recovery
- ✅ **Observability** — Prometheus metrics and structured logging
- ✅ **Quality** — verified algorithms, improved numerical precision, broad error handling

---

## Roadmap

- [x] Neuro-symbolic RAG core
- [x] Multi-agent architecture
- [x] SurrealDB integration with vector + graph
- [x] Interactive terminal UI
- [x] HTTP/WebSocket API server
- [x] Web control plane (Next.js)
- [x] Live LLM provider & key management
- [x] Hybrid retrieval v2 (BM25 + Reciprocal Rank Fusion + structural reranking)
- [x] Query transformation (HyDE) + LLM reranking
- [ ] Corrective retrieval (CRAG/Self-RAG) + self-consistency code generation
- [ ] Z3 symbolic verification
- [ ] SymPy equation solving
- [ ] Multi-paper synthesis
- [ ] Fine-tuned domain models
- [ ] Plugin system

---

## Contributing

Contributions are welcome.

```bash
# Engine
cd Paper2Codes-Core
cargo build
cargo test
cargo fmt && cargo clippy -- -D warnings

# Control plane
cd Paper2Codes-WebUI
npm install
npm run lint
```

Open an issue to discuss substantial changes before sending a pull request.

---

## Citation

```bibtex
@software{paper2codes,
  title  = {Paper2Codes: A Neuro-Symbolic RAG Framework for Automated Code Generation},
  author = {Dirac Technologies},
  url    = {https://github.com/rismanmattotorang/Dirac-Paper2Codes}
}
```

---

## License

Released under the MIT License — see [LICENSE](LICENSE).

---

## About Dirac Technologies

**Dirac Technologies** builds tools that close the gap between research and
working software. Paper2Codes is our reproduction layer for science: rigorous,
verifiable, and fast — engineering the bridge from ideas on paper to code in
production.

<div align="center">

**[⬆ back to top](#paper2codes)**

Made with ❤️ by **Dirac Technologies**

</div>
