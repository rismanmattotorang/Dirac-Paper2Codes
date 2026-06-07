import type { Page } from "@playwright/test"

/** Mock skills returned by the backend (shape matches `lib/api/skills.ts`). */
export const MOCK_SKILLS = [
  {
    id: "quantum-computation",
    name: "Quantum Computation",
    version: "1.0.0",
    description: "Quantum circuits, algorithms, and simulation.",
    domain: "Quantum Computing",
    keywords: ["qubit", "quantum circuit", "vqe"],
    languages: ["python"],
    libraries: { python: ["qiskit", "cirq", "pennylane", "qutip"] },
    instructions: "Implement quantum algorithms as circuits with explicit registers.",
    verification_hints: ["quantum states remain normalised"],
    builtin: true,
  },
  {
    id: "computational-finance",
    name: "Computational Finance",
    version: "1.0.0",
    description: "Pricing, risk, portfolio optimisation.",
    domain: "Computational Finance",
    keywords: ["option pricing", "monte carlo", "volatility"],
    languages: ["python", "cpp"],
    libraries: {
      python: ["numpy", "pandas", "scipy", "QuantLib"],
      cpp: ["QuantLib", "Eigen"],
    },
    instructions: "Implement quantitative-finance methods with numerical care.",
    verification_hints: ["no-arbitrage / put-call parity holds"],
    builtin: true,
  },
]

/** Provider key statuses returned by `GET /api/settings/llm/providers`. */
export const MOCK_PROVIDERS = [
  {
    id: "openai",
    name: "OpenAI",
    enabled: true,
    is_default: true,
    base_url: "https://api.openai.com/v1",
    models: ["gpt-4-turbo"],
    configured: true,
    masked_key: "sk-p…wxyz",
    key_source: "config",
    key_prefix_hint: "sk-",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    enabled: true,
    is_default: false,
    base_url: "https://openrouter.ai/api/v1",
    models: ["openai/gpt-4-turbo"],
    configured: true,
    masked_key: "sk-o…lmno",
    key_source: "config",
    key_prefix_hint: "sk-or-",
  },
  {
    id: "anthropic",
    name: "Anthropic",
    enabled: false,
    is_default: false,
    base_url: "https://api.anthropic.com/v1",
    models: ["claude-3-opus"],
    configured: false,
    key_source: "none",
    key_prefix_hint: "sk-ant-",
  },
  {
    id: "xai",
    name: "xAI (Grok)",
    enabled: false,
    is_default: false,
    base_url: "https://api.x.ai/v1",
    models: ["grok-2"],
    configured: false,
    key_source: "none",
    key_prefix_hint: "xai-",
  },
]

/** A structurally complete settings payload for `GET /api/settings`. */
export const MOCK_SETTINGS = {
  general: {
    organization_name: "Dirac Technologies",
    default_domain: "Auto-detect",
    dark_mode: false,
    theme: "default",
  },
  security: {
    two_factor_enabled: false,
    active_sessions: [
      {
        id: "s1",
        device: "Chrome on macOS",
        ip: "127.0.0.1",
        last_active: "Just now",
        created_at: "2026-01-01T00:00:00Z",
      },
    ],
    password_min_length: 8,
    jwt_expiration: 900,
    enable_csrf: true,
  },
  notifications: {
    paper_processing_complete: true,
    code_generation_errors: true,
    task_queue_updates: false,
    weekly_report: true,
  },
  llm: {
    primary_provider: "openai",
    api_key_configured: true,
    temperature: 0.7,
    model_preferences: {
      planning: "openai/gpt-4-turbo",
      analysis: "anthropic/claude-3-opus",
      coding: "openai/gpt-4-turbo",
      verification: "anthropic/claude-3-opus",
    },
    timeout_seconds: 300,
    max_retries: 3,
  },
  database: {
    connected: true,
    connection_string: "localhost:8000",
    namespace: "paper2codes",
    database: "main",
    max_connections: 10,
    backup_schedule: "Daily at 2:00 AM UTC",
    last_sync: "2026-01-01T00:00:00Z",
    schema_valid: true,
  },
  team: {
    members: [
      { id: "u1", name: "Current User", email: "user@dirac.id", role: "Admin", status: "Active" },
    ],
  },
}

/**
 * Install backend mocks so the Web UI runs without the Rust API or API keys.
 * Registered generic-first so the specific handlers take precedence (Playwright
 * runs the most-recently-registered matching route).
 */
export async function mockApi(page: Page): Promise<void> {
  // Permissive fallback for any backend call the UI fires on load.
  await page.route("**/api/**", async (route) => {
    await route.fulfill({ json: {} })
  })
  // Skills listing.
  await page.route("**/api/skills", async (route) => {
    await route.fulfill({ json: MOCK_SKILLS })
  })
  // Single-skill detail (used by the selected-skill banner).
  await page.route("**/api/skills/*", async (route) => {
    const match = route.request().url().match(/skills\/([^/?]+)/)
    const id = match ? decodeURIComponent(match[1]) : ""
    const skill = MOCK_SKILLS.find((s) => s.id === id) ?? MOCK_SKILLS[0]
    await route.fulfill({ json: skill })
  })
}

/** Mock the settings + LLM-key endpoints (call after {@link mockApi}). */
export async function mockSettingsApi(page: Page): Promise<void> {
  await page.route("**/api/settings", async (route) => {
    await route.fulfill({ json: MOCK_SETTINGS })
  })
  await page.route("**/api/settings/llm/providers", async (route) => {
    await route.fulfill({ json: MOCK_PROVIDERS })
  })
  // Live key validation.
  await page.route("**/api/settings/llm/providers/*/test", async (route) => {
    await route.fulfill({
      json: { valid: true, message: "Key is valid (HTTP 200).", latency_ms: 9 },
    })
  })
  // Set / update a provider key → return it as configured.
  await page.route("**/api/settings/llm/providers/*/key", async (route) => {
    const match = route.request().url().match(/providers\/([^/]+)\/key/)
    const id = match ? decodeURIComponent(match[1]) : "unknown"
    const base = MOCK_PROVIDERS.find((p) => p.id === id) ?? {
      id,
      name: id,
      enabled: true,
      is_default: false,
      base_url: null,
      models: [],
      key_prefix_hint: null,
    }
    await route.fulfill({
      json: { ...base, configured: true, enabled: true, masked_key: "sk-…test", key_source: "config" },
    })
  })
  // Set default provider → return the list with the new default.
  await page.route("**/api/settings/llm/default", async (route) => {
    let provider = ""
    try {
      provider = (route.request().postDataJSON() as { provider?: string })?.provider ?? ""
    } catch {
      provider = ""
    }
    const updated = MOCK_PROVIDERS.map((p) => ({ ...p, is_default: p.id === provider }))
    await route.fulfill({ json: updated })
  })
}
