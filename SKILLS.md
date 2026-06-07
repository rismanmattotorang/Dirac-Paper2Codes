# Domain Skills

Dirac-Paper2Codes ships **Domain Skills** — modular, reusable, user-editable
bundles of domain expertise that specialise the paper→code pipeline for a
computational field. The design follows the *Agent Skills* convention (a name +
trigger description + a procedural body, plus structured metadata), adapted to
scientific code generation.

Selecting a skill specialises three stages:

- **Retrieval** — the skill's keywords bias the hybrid retriever toward the
  domain's vocabulary.
- **Generation** — the skill's expertise primer and language-specific library
  recommendations are injected into the coding prompt.
- **Verification** — the skill's domain invariants (e.g. energy conservation,
  no-arbitrage, normalised quantum states) are surfaced as checks/hints.

## Main scenario

1. **Choose** a domain skill (Web UI → *Domain Skills*, or TUI view `7`).
2. **Pick a language** offered by that skill.
3. **Upload a paper**.
4. The engine performs **RAG** (BM25 + dense + RRF, structural reranking, HyDE,
   CRAG expansion) biased by the skill, then **generates** domain-idiomatic code.
5. The result is **evaluated** (SACV verification + optional benchmark scoring).

## Built-in skills

| Skill id | Domain | Example libraries (Python) |
|---|---|---|
| `computational-finance` | Computational Finance | numpy, pandas, scipy, QuantLib, cvxpy |
| `computational-physics` | Computational Physics | numpy, scipy, sympy |
| `computational-chemistry` | Computational Chemistry | rdkit, ase, pyscf, openmm |
| `computational-biology-bioinformatics` | Comp. Biology & Bioinformatics | biopython, pysam, scikit-bio |
| `computational-genomics` | Computational Genomics | pysam, scikit-allel, biopython |
| `quantum-computation` | Quantum Computation | qiskit, cirq, pennylane, qutip |
| `computational-fluid-dynamics` | CFD | numpy, scipy, fenics, fipy |
| `computational-supply-chain` | Supply Chain Management | pulp, ortools, pyomo, networkx, simpy |

Each skill also declares C++/Rust libraries where idiomatic, trigger keywords,
and verification properties.

## Skill anatomy

A skill is the following structure (serialised as TOML for user skills):

```toml
id = "computational-finance"
name = "Computational Finance"
version = "1.0.0"
description = "Quantitative finance: pricing, risk, portfolio optimisation."
domain = "Computational Finance"
keywords = ["option pricing", "black-scholes", "monte carlo", "volatility"]
languages = ["python", "cpp", "rust"]
instructions = "Implement quantitative-finance methods with numerical care..."
verification_hints = ["no-arbitrage / put-call parity holds where applicable"]

[libraries]
python = ["numpy", "pandas", "scipy", "QuantLib"]
cpp = ["QuantLib", "Eigen", "Boost"]
rust = ["ndarray", "statrs"]
```

## Choosing, reusing, and improving skills

- **Choose** — `GET /api/skills` lists every skill; the Web UI *Domain Skills*
  page lets you browse, inspect, pick a language, and select one for your next
  paper (persisted client-side).
- **Reuse** — built-in skills ship with the engine and are always available.
- **Improve** — author or override a skill by `PUT /api/skills/:id` (persisted to
  the user skills directory `<config>/paper2codes/skills/<id>.toml`), or simply
  drop a `<id>.toml` file there. User skills override built-ins with the same id.

## API

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/skills` | List all skills (built-in + user) |
| `GET` | `/api/skills/:id` | Fetch one skill |
| `PUT` | `/api/skills/:id` | Create or improve a user skill |

## Library API (Rust)

```rust
use paper2codes::skills::SkillRegistry;

let mut registry = SkillRegistry::with_builtins();
registry.load_user_dir(SkillRegistry::user_skills_dir()?)?; // overlay user skills

let skill = registry.get("quantum-computation").unwrap();
let primer = skill.system_prompt("python");       // domain prompt for the coder
let libs = skill.libraries_for("python");          // idiomatic libraries

// Auto-suggest a skill from the paper's text:
let suggested = registry.match_skill("a variational quantum eigensolver ansatz");
```

The selection → generation → evaluation flow is covered end-to-end in
`Paper2Codes-Core/tests/skills_e2e.rs`.
