# Dirac-Paper2Codes — Path to Production

*Owner: Dirac Technologies (Dirac.id) · Living document*

This plan takes Dirac-Paper2Codes from its current state — **engineering-complete
and CI-green, but not yet validated against live LLMs or hardened for
production** — to a reliable, secure, observable production service.

---

## 1. Where we are today (honest baseline)

**Built and CI-verified (offline):**
- Rust engine (axum API, coordinator, multi-agent pipeline), Next.js Web UI, Ratatui TUI.
- Hybrid retrieval (BM25 + RRF + structural rerank + HyDE + LLM rerank + CRAG), self-consistency, self-debugging, graph-native ordering.
- 8 domain Skills (choose/reuse/improve), wired into live `process_paper`.
- Live LLM key management (hot reload, masked, `0600` at rest).
- Benchmark harness + 8-domain offline dataset; `paper2codes bench [--offline]`.
- CI (`ci.yml`): Rust build/tests/offline-bench + Web UI typecheck + Playwright e2e, all green.

**Not yet done / explicitly gated (the gap to production):**
- ❗ **No live-LLM validation** — real code-generation quality is unmeasured (no API keys exercised end-to-end).
- ❗ **Untrusted code execution** — verification runs generated code; needs strong sandbox isolation.
- ❗ **Durable job execution** — generation is an in-process `tokio::spawn`; work is lost on restart.
- ❗ **Auth/secrets hardening** — default `jwt_secret = "change-me-in-production"`, default SurrealDB `root/root`, plaintext keys in `config.toml`; 2FA/sessions/team are mocked.
- ❗ **Release/deploy pipeline** — Dockerfiles/compose/k8s exist but are unhardened (no TLS, secrets, autoscaling, image publishing).
- ❗ **Observability** — Prometheus metrics + structured logs exist; no dashboards/alerts/tracing.
- ❗ **Generated-repo UX** — the "Generated Code" page isn't bound to the processed paper/run.

---

## 2. Production principles & SLOs (targets to agree on)

- **Availability:** 99.5% for the API/UI (single region to start).
- **Latency:** p95 < 300 ms for CRUD/API; generation is async (minutes) with progress streaming.
- **Correctness gate:** a tagged release must pass the live benchmark above an agreed score floor (see §7).
- **Security:** least privilege, secrets never in source/images, untrusted code never on the host network or filesystem.
- **Cost:** per-paper LLM spend bounded and observable; per-tenant quotas.

---

## 3. Phased plan

### Phase 0 — Validation & correctness (unblocks everything)
**Goal:** prove the engine actually produces quality code with real keys.

*Tooling shipped (this phase):*
- ✅ **LLM-judge rubric scoring** (`--rubric`, reference-free 0–1 faithfulness).
- ✅ **Quality gate** (`--min-reference-overall`, `--min-rubric`, `--baseline`) that exits non-zero on a floor breach or regression — ready to gate releases (§7).
- ✅ **Nightly benchmark workflow** (`.github/workflows/benchmark.yml`): live when an API-key secret is set, offline smoke otherwise; uploads `report.json`.
- ✅ Dataset expanded to **25 cases** across all 8 domains (living artifact).
- ✅ Gate auto-enables in the nightly workflow once `bench/baseline.json` is committed (+ optional floors via repo variables).

*Remaining (needs keys / human review):*
- Provision LLM keys (OpenAI/Anthropic/OpenRouter) in a **secret store**, not config files.
- Run `paper2codes bench --rubric` (live); record per-domain file-F1, content similarity, and rubric score; commit the first reviewed report as `bench/baseline.json`.
- Establish the **score floor** from the first live runs (set `BENCH_MIN_*` repo variables); the gate then enforces it automatically.
- Optionally add harder multi-file references / wire **PaperBench** for scale.
- **Exit criteria:** reproducible benchmark report; agreed score floor; ≥1 end-to-end paper→repo run reviewed by a domain expert.

### Phase 1 — Security & secrets hardening
- Replace `config.toml` plaintext keys with a **secret manager** (Vault / cloud KMS / k8s Secrets); encrypt-at-rest; rotate.
- Generate a strong `jwt_secret` per environment; enforce non-default at boot (fail closed).
- Real **auth**: finish password policy, sessions, and 2FA (currently mocked); add RBAC and per-user API tokens; fix Web UI token handling (no `localStorage` TODO).
- Harden SurrealDB: non-default credentials, least-privilege user, network isolation, TLS.
- Dependency & supply-chain scanning in CI: `cargo audit`, `pnpm audit`, SAST (e.g. CodeQL), and an SBOM per release.
- **Exit criteria:** secret-scan clean; no default credentials anywhere; auth flows tested; threat model documented.

### Phase 2 — Safe, durable execution
- **Sandbox** for generated-code verification: isolate with gVisor/Firecracker or rootless containers; no host FS, no outbound network by default, CPU/mem/time limits, seccomp. (Today's `bollard`/docker path must be treated as untrusted-code execution.)
- **Durable job queue**: move generation off in-process `tokio::spawn` to a persisted task/worker model (DB-backed queue or Redis/NATS) so runs survive restarts, can retry, and scale horizontally. Persist task state + progress; resume/cancel.
- **Cost controls**: per-run token budgets, per-tenant rate limits/quotas, circuit breakers on provider errors, spend metrics.
- **Exit criteria:** generated code can never touch the host; a worker crash/restart loses no work; a runaway paper can't exceed its budget.

### Phase 3 — Generated-artifact UX & data lifecycle
- Bind the Web UI **Generated Code** page to the processed paper/run; stream progress (already have WebSocket plumbing) and surface verification results.
- Repository persistence + download (zip already exists) tied to paper + run id; versioning.
- Data retention/privacy: uploaded papers may be copyrighted/PII — define retention, deletion, and access policies.
- **Exit criteria:** upload → watch progress → browse/download verified repo, all in the UI for a real run.

### Phase 4 — Deploy & infrastructure
- Harden Dockerfiles (distroless/non-root, pinned digests); **publish images** from CI on tags.
- Modernize k8s manifests/Helm: secrets, TLS/ingress, readiness/liveness probes, HPA, resource requests/limits, PodSecurity.
- Environments: dev → staging → prod with promotion; IaC (Terraform) for cloud resources + managed SurrealDB/Postgres + secret store.
- Blue/green or canary deploys; documented rollback.
- **Exit criteria:** one-command deploy to staging; automated image publish on tag; TLS + secrets in cluster; rollback rehearsed.

### Phase 5 — Observability & SRE
- Dashboards (Grafana) for API latency, generation throughput/success, LLM tokens/cost, queue depth; the repo already ships Prometheus + a Grafana scaffold.
- **Alerting** (error rate, latency, queue backlog, provider failures, budget breach) → on-call.
- **Distributed tracing** (OpenTelemetry) across API → coordinator → agents → LLM/storage.
- Runbooks (incident response, key rotation, DB restore), load tests, and a tested backup/restore.
- **Exit criteria:** golden-signal dashboards + alerts live; trace of a full generation; restore drill passed.

### Phase 6 — Beta → GA
- Private beta with real users + papers; collect quality/satisfaction + cost telemetry.
- Make the lint job a **required** gate (clean `cargo fmt`/`clippy` pass first), and the live benchmark a release gate.
- Docs: API reference (OpenAPI already present), user guide, SLA, pricing/quota policy.
- **Exit criteria:** SLOs met for 2+ weeks in staging/beta; security review signed off; GA runbook complete.

---

## 4. Suggested sequence & rough effort
1. **Phase 0** (validation) — *now*, days. Highest information value; do first.
2. **Phase 1 + 2** in parallel (security + safe/durable execution) — weeks. Hard prerequisites for any public exposure.
3. **Phase 3** (artifact UX) — alongside 1/2.
4. **Phase 4 + 5** (deploy + observability) — weeks.
5. **Phase 6** (beta→GA) — ongoing.

## 5. Top risks
| Risk | Mitigation |
|---|---|
| Untrusted generated code executes on host | Phase 2 sandbox; no-network default; resource caps |
| LLM quality below expectations | Phase 0 measurement + score floor gate before GA |
| Secret leakage | Phase 1 secret manager, scanning, fail-closed on defaults |
| Lost/duplicated work on restart | Phase 2 durable queue + idempotent tasks |
| Runaway LLM cost | Per-run budgets, quotas, spend alerts |
| Single-region outage | Start single-region with backups; multi-region post-GA |

## 6. Definition of "production ready"
All of: Phase 0 benchmark floor met · no default secrets/credentials · sandboxed untrusted execution · durable job queue · TLS + secret-managed deploy on staging · golden-signal dashboards + alerts · backup/restore drill passed · security review signed off.

## 7. Quality gate (concrete)
- Live `paper2codes bench` over the curated dataset must meet/exceed the agreed mean reference-overall and rubric thresholds, with **no regression** vs. the previous tagged release, before promoting to prod.

---

*This complements [STRATEGY.md](STRATEGY.md) (capability roadmap) and
[TESTING.md](TESTING.md) (how to validate offline now and live once keyed).*
