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

/**
 * Install backend mocks so the Web UI runs without the Rust API or API keys.
 * Registered generic-first so the specific `/api/skills` handler takes
 * precedence (Playwright runs the most-recently-registered matching route).
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
}
