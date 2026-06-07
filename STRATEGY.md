# Dirac-Paper2Codes — Competitive Benchmark & Superiority Strategy

*Owner: Dirac Technologies · Status: living document*

This document benchmarks the Dirac-Paper2Codes engine against comparable
open-source systems, identifies concrete gaps, and lays out a phased strategy
to make Dirac-Paper2Codes decisively superior in **features** and **code
generation quality**. It is grounded in (a) a close read of the two source
papers (`Paper2Codes.tex`, `CoScientist.tex`) and the actual engine source, and
(b) a survey of current RAG and code-generation literature.

---

## 1. Competitive landscape

| System | Scope | Pipeline | Retrieval | Verification | Multi-LLM | Persistence | UI |
|---|---|---|---|---|---|---|---|
| **Dirac-Paper2Codes** | Any-domain papers → repo | Plan → Analyze → Code → **Verify** (iterative) | Hybrid CPR (dense + BM25 + RRF + MMR + structural) | **SACV**: static + dynamic + symbolic | ✅ OpenAI/Anthropic/OpenRouter/xAI, live routing | ✅ SurrealDB (doc+graph+vector) | ✅ Next.js + TUI |
| **Paper2Code / PaperCoder** (KAIST, arXiv 2504.17192) | ML papers → repo | Plan → Analyze → Generate | Prompt-context, no formal hybrid retriever | ❌ (no exec/formal verify) | ❌ single model | ❌ | ❌ |
| **MetaGPT** | General software (SOP) | PM→Arch→Eng→QA | ❌ | tests only | ❌ | ❌ | partial |
| **ChatDev** | General software (chat) | role dialogue | ❌ | tests only | ❌ | ❌ | ❌ |
| **AutoCodeRover / SWE-agent / OpenHands** | Repo issue fixing (SWE-bench) | localize → patch → test | code search (AST/BM25) | tests / CI | varies | repo | varies |

**Takeaways.**
- Dirac-Paper2Codes is already **architecturally ahead** of the closest
  competitor (Paper2Code): it adds neuro-symbolic verification, multi-domain
  handling, multi-LLM routing, a unified SurrealDB store, and a web UI.
- The competitors' main advantage is **benchmark rigor**: Paper2Code is
  evaluated on **PaperBench** with author-as-judge protocols. Dirac should adopt
  a comparable harness to *prove* superiority, not just claim it.
- Issue-fixing agents (SWE-agent et al.) are not direct competitors but are the
  source of the best **self-repair / execution-feedback** techniques, which we
  should absorb.

---

## 2. Gap analysis (paper claims vs. shipped code)

A close read surfaced several places where the implementation lagged the paper's
own description — the highest-leverage fixes because they restore *claimed*
capability:

1. **Retrieval lexical signal** — *Paper claims BM25; code used a substring
   containment ratio.* No term-frequency saturation, no IDF, no length
   normalisation. **(Fixed — Phase 1.)**
2. **Dense + sparse fusion** — *Code summed a cosine score (`[-1,1]`) and a
   keyword ratio (`[0,1]`) with fixed weights.* Different scales → the larger
   scale dominates. The literature standard is **Reciprocal Rank Fusion**.
   **(Fixed — Phase 1.)**
3. **Structural markers** — *Paper highlights algorithm boxes / equation labels
   as high-precision targets; code only checked for the word "algorithm".*
   **(Fixed — Phase 2: exact "Algorithm 1" / "Equation (5)" matching.)**
4. **No query transformation** — no HyDE / query expansion. *(Phase 3.)*
5. **No learned reranking** — only MMR diversity; no cross-encoder / LLM rerank.
   *(Phase 3.)*
6. **No corrective/self-reflective retrieval** — retrieval is single-shot.
   *(Phase 4: CRAG / Self-RAG.)*
7. **Code-gen quality** — single-sample generation; no self-consistency voting
   or self-debugging loop closed on *self-generated* tests. *(Phase 4.)*
8. **No reproducibility benchmark harness.** *(Phase 5.)*

---

## 3. Strategy — phased roadmap

### Phase 1 — Hybrid retrieval v2: BM25 + RRF  ✅ *shipped in this PR*
Proper Okapi BM25 (`retrieval/bm25.rs`) + Reciprocal Rank Fusion
(`retrieval/fusion.rs`), fused with the dense ranking inside `DefaultCPREngine`.
Rationale: dense and sparse retrievers fail in orthogonal ways; RRF fusion is
unsupervised, scale-agnostic, and reported to add up to **+8pp Recall@5** over
either retriever alone. Closes gaps #1, #2.

### Phase 2 — Structural-reference reranking  ✅ *shipped in this PR*
Regex extraction of structural anchors ("Algorithm 1", "Equation (5)",
"Theorem 2", "Figure 3", "Section 4.2") from the task, with exact-match boosting
of segments that declare the same anchor. High-precision, deterministic,
testable. Closes gap #3.

### Phase 3 — Query transformation + learned reranking  ✅ *shipped in this PR*
- **HyDE** (`retrieval/augment.rs`): an LLM writes a short *hypothetical
  implementation sketch* for the task; we embed it and blend it with the
  original-query embedding (`combine_embeddings`, L2-renormalised) before dense
  search — bridging the query↔document vocabulary gap while a poor hypothesis
  cannot fully derail retrieval.
- **LLM reranker** (`retrieval/augment.rs`): the fused top-N candidates are
  re-ordered by an LLM acting as a cross-encoder-style reranker; the new order
  re-derives relevance that drives MMR selection. Response parsing
  (`parse_rerank_order`) is forgiving and always yields a complete permutation.
- **Capability-gated**: HyDE needs an embedding key, reranking needs an LLM
  client; both fall back cleanly to the Phase 1+2 pipeline when absent. Wired in
  the `Coordinator` and isolated behind the `QueryExpander` / `Reranker` traits
  so the deterministic pieces are unit-tested and the LLM calls are mockable.

### Phase 4 — Corrective retrieval + code-gen quality  ✅ *shipped in this PR*
- **CRAG corrective retrieval** (`retrieval/crag.rs`): grades retrieval
  confidence from the top relevance scores and, when weak, performs deterministic
  knowledge expansion — widening the selected context (`Ambiguous → k+⌈k/2⌉`,
  `Incorrect → 2k`) instead of returning a thin, possibly-wrong top-k. Wired into
  `DefaultCPREngine::retrieve`.
- **Self-consistency selection** (`evaluation/self_consistency.rs`): a
  deterministic best-of-N policy (verification pass → pass-rate → judge score →
  majority agreement → stable index) plus `best_of_n`, an async driver over a
  mockable `CandidateGenerator`. Verifier-guided selection lifts correctness over
  single-sample generation.
- **Self-debugging repair** (`agents/repair.rs`): groups a module's verification
  issues into one severity-prioritised, rubber-duck-style repair brief
  (Chen et al., 2023) with a capped-iteration policy; wired into the
  coordinator's `handle_verification_feedback` (one structured fix task per
  module instead of one per symptom).

### Phase 5 — Reproducibility benchmark harness  ✅ *shipped in this PR*
- `benchmark/` module + `bench/` manifest dir + a `paper2codes bench` CLI
  subcommand. Runs a curated paper→repo set (PaperBench-style) and scores
  generated repos two ways:
  - **reference-based** (`scoring::reference_score`): file precision/recall/F1 by
    normalised filename + token-Jaccard content similarity over matched files;
  - **reference-free**: optional LLM-judge rubric via the `RubricGrader` trait.
- Results aggregate into a `BenchmarkReport` (Markdown + JSON). `RepoGenerator` /
  `RubricGrader` are traits, so the runner is unit-tested with mocks while the
  CLI wraps the real coordinator. Makes "superior" measurable and regression-safe.

### Phase 6 — Graph-native retrieval (leverage SurrealDB)
- **GraphRAG over the dependency graph**: use SurrealDB's graph edges
  (module/citation dependencies) to expand retrieval along structural relations,
  and to order generation by topological dependency — a differentiator no
  competitor has, since they lack a unified graph+vector store.

---

## 4. References

- Cormack, Clarke, Büttcher. *Reciprocal Rank Fusion outperforms Condorcet and
  individual Rank Learning Methods.* SIGIR 2009.
- Robertson, Zaragoza. *The Probabilistic Relevance Framework: BM25 and Beyond.* 2009.
- Gao et al. *Precise Zero-Shot Dense Retrieval without Relevance Labels* (HyDE). 2022.
- Seo et al. *Paper2Code: Automating Code Generation from Scientific Papers in
  Machine Learning.* arXiv:2504.17192, 2025.
- Shinn et al. *Reflexion: Language Agents with Verbal Reinforcement Learning.* 2023.
- Chen et al. *Teaching Large Language Models to Self-Debug.* 2023.
- Asai et al. *Self-RAG.* 2023 · Yan et al. *Corrective RAG (CRAG).* 2024.
- Hong et al. *MetaGPT.* 2024 · Anthropic. *Contextual Retrieval.* 2024.
