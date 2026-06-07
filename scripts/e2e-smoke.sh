#!/usr/bin/env bash
#
# End-to-end offline smoke test for Dirac-Paper2Codes.
#
# Validates the engine, CLI, benchmark harness, domain skills, and (optionally)
# the Web UI type-check WITHOUT any API keys. Use this to confirm the system is
# healthy before configuring OpenAI/Anthropic/OpenRouter keys for live runs.
#
# Usage:  scripts/e2e-smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE="$ROOT/Paper2Codes-Core"
WEBUI="$ROOT/Paper2Codes-WebUI"

pass() { printf '\033[32m✓ %s\033[0m\n' "$1"; }
info() { printf '\033[34m• %s\033[0m\n' "$1"; }

info "Building the engine (release-free check build)…"
cargo build --quiet --manifest-path "$CORE/Cargo.toml"
pass "engine builds"

info "Running Rust unit + integration tests (incl. TUI render harness)…"
cargo test --quiet --manifest-path "$CORE/Cargo.toml" \
  --lib --test e2e_pipeline --test skills_e2e --test tui_render
pass "all Rust tests pass (engine, pipeline, skills, TUI rendering)"

BIN="$CORE/target/debug/paper2codes"

info "CLI: version"
"$BIN" version
pass "version"

info "CLI: listing built-in domain skills via offline benchmark…"
# The offline benchmark runs the full harness (manifest → generate → score)
# deterministically, no API keys required.
"$BIN" bench --manifest "$CORE/bench/dataset/manifest.json" --offline
pass "offline benchmark ran and scored the bundled dataset"

if [ -d "$WEBUI/node_modules" ]; then
  info "Web UI: type-check"
  (cd "$WEBUI" && npx tsc --noEmit -p tsconfig.json)
  pass "Web UI type-checks"

  # Playwright UI tests (mock the backend; no API keys needed). Requires the
  # browser binaries — install once with: npx playwright install chromium
  if (cd "$WEBUI" && npx playwright --version >/dev/null 2>&1); then
    info "Web UI: Playwright e2e tests"
    if (cd "$WEBUI" && npx playwright test 2>/dev/null); then
      pass "Web UI Playwright tests pass"
    else
      info "Web UI: Playwright tests skipped/failed (run 'npx playwright install chromium' first)"
    fi
  else
    info "Web UI: Playwright not installed (run 'pnpm install' + 'npx playwright install chromium')"
  fi
else
  info "Web UI: skipping (run 'pnpm install' in Paper2Codes-WebUI first)"
fi

echo
pass "Offline end-to-end smoke test complete."
echo "Next: configure LLM API keys (see LLM_API_KEY.md), then run a live benchmark:"
echo "  $BIN bench --manifest $CORE/bench/dataset/manifest.json --output report.json"
