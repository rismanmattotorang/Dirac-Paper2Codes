# Dirac-Paper2Codes — Final Capability Assessment (v1.0)

*Owner: Dirac Technologies (Dirac.id). Last validated: 2026-06-08.*

This document is an **evidence-based** audit of every headline engine and
platform capability: what is implemented, to what depth, how it was validated,
and where it stands relative to the state of the art. Verdicts are sourced from
the code and from runnable checks — not from marketing copy. Where a capability
is partial or pending live validation, we say so plainly.

## How this was validated

| Check | Result |
|---|---|
| Rust core unit/integration tests (`cargo test`) | **130 lib tests + PDF-extraction integration tests pass, 0 failures** |
| Web UI type safety (`tsc --noEmit`) | **clean** |
| Reproducibility benchmark, offline (`paper2codes bench --offline`, 25 cases / 8 domains) | **25/25 scored, 0 errors; mean file-structure F1 = 1.000** |
| Real PDF text extraction (bundled `Dirac-Paper2Codes.pdf`, `Dirac-CoScientist.pdf`) | **140 KB extracted; title/authors/sections recovered** |

> **Honesty note on quality.** The offline benchmark uses a deterministic stub
> generator, so it proves the *harness, dataset, scoring, and quality gate* work
> end-to-end and that generated repositories have the correct file structure
> (F1 = 1.0). It does **not** measure LLM code-generation quality — that needs
> live provider keys. The rubric grader and the regression-gating harness are in
> place to measure it the moment keys are provisioned (see `PRODUCTION_PLAN.md`).

---

## Engine

| Capability | Verdict | Evidence |
|---|---|---|
| **Intelligent paper processing** — parse PDFs and text; extract algorithms, equations, structure | **Implemented** | Real PDF text extraction via the `pdf` crate's content-operator stream (`Tj`/`TJ`/`T*`), validated against the two bundled papers (`src/document/parser.rs`); algorithm/equation/structure extraction in `src/document/extractor.rs`, `segmenter.rs`. |
| **Automatic domain classification** | **Implemented** | LLM-based detector across 14 computational domains with a rule-based fallback (`src/domain/detector.rs`), wired into the processing pipeline (`src/document/mod.rs`). |
| **Multi-agent orchestration** — semaphore concurrency, dependency resolution, convergence detection | **Implemented** | `tokio::Semaphore`-backed adaptive concurrency (`src/performance/concurrency.rs`), Kahn topological ordering (`src/storage/graph.rs`), and a 6-criterion convergence check (`src/coordinator/mod.rs`). |
| **Advanced retrieval** — embedding vector search (OpenAI, Voyage AI) + hybrid CPR scoring | **Implemented** | OpenAI + Voyage embedding providers (`src/retrieval/embedding.rs`); BM25 + Reciprocal Rank Fusion + HyDE + LLM rerank + CRAG + MMR in the CPR pipeline (`src/retrieval/mod.rs`, `bm25.rs`, `fusion.rs`); degrades gracefully to BM25 when no embedding key is present. |
| **SACV verification** — static, dynamic, symbolic in one pipeline | **Implemented (symbolic tier is lightweight)** | `SACVPipeline` runs static (tree-sitter syntax/logic, `src/verification/static_analysis.rs`) → dynamic (test generation + sandboxed execution, `dynamic_tests.rs`) → symbolic. The symbolic tier is a **built-in linear-constraint SMT** (`src/symbolic/smt.rs`), not Z3; full Z3/SymPy back-ends are on the roadmap. |
| **Persistent storage** — SurrealDB, native vector search, graph dependency tracking | **Implemented** | SurrealDB client with health-checked reconnection (`src/storage/surreal.rs`), native `vector::similarity::cosine` search, and graph dependency analysis/ordering (`src/storage/graph.rs`). Users, sessions, API tokens, and the job queue are now **durably persisted** with write-through + load-on-restart. |

---

## Platform

| Capability | Verdict | Evidence |
|---|---|---|
| **Domain Skills** — choose/reuse/improve a domain specialisation | **Implemented** | All 8 domains (Finance, Physics, Chemistry, Biology/Bioinformatics, Genomics, Quantum, CFD, Supply Chain) in `src/skills/builtin.rs`; skills augment generation prompts, bias retrieval (keyword overlap), and inject verification hints; user-authored TOML overrides; surfaced in both Web UI and TUI. |
| **Live LLM key management** | **Implemented** | Per-provider set/test/rotate/remove with masked display, source tracking, and immediate hot-reload (`src/api/handlers/llm_keys.rs`, `components/settings/api-keys-panel.tsx`); keys never returned in full; persisted at `0600`. |
| **Dual interface** — browser control plane + interactive Ratatui TUI | **Implemented** | A full 11-view Ratatui TUI that drives the engine, not a shell (`src/ui/app.rs`), render-tested via `TestBackend` (`tests/tui_render.rs`); Next.js control plane in `Paper2Codes-WebUI`. |
| **Real-time updates** — REST + WebSocket streaming | **Implemented** | WebSocket manager + typed protocol with JWT-gated upgrade (`src/api/websocket/`) and SSE progress/log streaming (`src/api/handlers/streaming.rs`). |
| **Production hardening** — JWT, rate limiting, validation, structured logging, Prometheus | **Implemented** | JWT auth with RBAC (`src/api/auth/`), `governor` per-endpoint rate limits, `validator` request validation, `tracing` structured logs, and a Prometheus metrics registry (`src/api/metrics.rs`). Fail-closed security validation rejects default secrets in production. |
| **Resilience** — retry with exponential backoff, graceful degradation | **Implemented** | LLM router retry loop with `2^attempt` backoff and retryable-error detection, multi-provider fallback chains, per-run token budget (`src/llm/router.rs`); retrieval and generation degrade gracefully without keys/DB; the durable job queue recovers `Running` work after a crash. |

---

## Where we stand vs. the state of the art

The defensible, architecture-level differentiators — each verified above — are:

1. **Grounded generation, not single-shot prompting.** Most "paper → code" tools
   are a single LLM call over a prompt. Dirac-Paper2Codes runs a **hybrid
   retrieval** stage (dense + BM25 + RRF + HyDE + rerank + CRAG) that keeps each
   module anchored to the cited text, plus structural-reference boosting that
   vanilla RAG lacks.
2. **A verification pipeline, not just generation.** Output passes static →
   dynamic → symbolic checks before it is called "done" — a closed loop with
   self-debugging repair, where typical code-gen stops at first output.
3. **Domain specialisation as a first-class, reusable artifact.** Eight
   computational-domain skills shape retrieval, generation, and verification and
   are user-improvable — beyond generic prompt templates.
4. **Durable, observable execution.** Generation runs on a crash-recoverable
   durable job queue with progress, retries, cancellation, and Prometheus
   metrics — production traits absent from notebook-grade tools.
5. **Two real interfaces over one Rust engine** — a browser control plane and a
   full TUI — with live, server-side LLM-key management.

**What "superior" requires next (intellectually honest):** a *quantitative*
head-to-head against named systems on code-generation **quality** is not yet
claimed, because it requires live LLM keys. The machinery to prove it is built
and shipping — the reproducibility harness, the 25-case/8-domain dataset, the
LLM-judge rubric grader, and a release **quality gate** that fails on regression
(`paper2codes bench --rubric --baseline …`). The path from "architecturally
superior and fully built" to "measured superior" is: provision keys → run the
live benchmark → commit the baseline → enforce the gate. This is the top item in
`PRODUCTION_PLAN.md` Phase 0.

## Known gaps (tracked)

- **Symbolic verification** is a built-in linear-constraint solver; Z3 and SymPy
  back-ends are roadmap items.
- **Live-LLM quality** is unmeasured pending keys (harness + gate ready).
- **2FA (TOTP)** is deferred (no OTP dependency vendored); the data model and
  Web UI Security tab are ready to host it.
- **Scanned/image-only PDFs** need OCR; text-based PDFs are fully supported and
  validated.

---

*See [PRODUCTION_PLAN.md](PRODUCTION_PLAN.md) for the phased path to a measured,
deployed GA, and [STRATEGY.md](STRATEGY.md) for the capability roadmap.*
