# Testing Dirac-Paper2Codes

Dirac-Paper2Codes is designed to be **fully testable offline** — every feature
has deterministic tests that run without API keys. Live code-generation quality
is validated separately once LLM keys are configured.

## 1. Offline (no API keys) — run this now

A single command exercises the engine, CLI, benchmark harness, and domain skills,
and type-checks the Web UI:

```bash
scripts/e2e-smoke.sh
```

What it covers:

- **Engine build** (`cargo build`).
- **Unit + integration tests** (`cargo test --lib --test e2e_pipeline --test skills_e2e`):
  - hybrid retrieval (BM25 + RRF + structural reranking + CRAG),
  - HyDE / reranking helpers, self-consistency, self-debugging, graph ordering,
  - **offline paper parsing → segmentation → retrieval** on a real bundled paper,
  - **the full benchmark harness** over the bundled dataset,
  - the **domain-skill main scenario** (choose skill → skill-guided generation → evaluate),
  - all 8 built-in domain skills are well-formed.
- **CLI**: `paper2codes version` and an **offline benchmark** run
  (`paper2codes bench --offline`) that scores the bundled dataset with a
  deterministic stub generator — no network, no keys.
- **Web UI**: `tsc --noEmit` type-check (when `node_modules` is present).

### Manual UI checks

- **Web UI** — `cd Paper2Codes-WebUI && pnpm install && pnpm dev`, open
  <http://localhost:3000>: visit *Domain Skills* (browse/select a skill + language),
  *Settings → LLM Configuration* (manage provider keys), *Papers*, *Generated Code*.
- **TUI** — `paper2codes tui`; press `7` for the **Domain Skills** view, and the
  number/Tab keys to move between Dashboard/Tasks/Modules/Code/Skills/etc.

## 2. The benchmark dataset

`Paper2Codes-Core/bench/dataset/` ships a small, self-contained, domain-mapped
benchmark (no external/licensed papers required):

| Case | Skill | Reference |
|---|---|---|
| `black-scholes` | computational-finance | European option pricing |
| `newton-sqrt` | computational-physics | Newton's method for √a |
| `bell-state` | quantum-computation | Two-qubit Bell state |

Each case has a paper (`papers/*.txt`), an author-style reference repo
(`refs/*/main.py`), and an entry in `manifest.json`. Scoring is reference-based
(file F1 + content similarity); see [Paper2Codes-Core/bench/README.md](Paper2Codes-Core/bench/README.md).

For a **large external benchmark**, the standard is **PaperBench**
(Seo et al., *Paper2Code*, arXiv:2504.17192). Point a manifest at a PaperBench
checkout to evaluate at scale.

## 3. Live quality validation — once API keys are configured

1. Configure a provider key (see [LLM_API_KEY.md](LLM_API_KEY.md)) — via the Web
   UI *Settings*, `config.toml`, or env (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`,
   `OPENROUTER_API_KEY`).
2. Run the benchmark for real (drops `--offline`, uses the coordinator):

   ```bash
   paper2codes bench --manifest Paper2Codes-Core/bench/dataset/manifest.json --output report.json
   ```

3. Inspect `report.json` / the Markdown summary: per-case file F1, content
   similarity, and overall scores. Add your own cases (with `skill_id`) to the
   manifest to measure domain-specific generation quality.
