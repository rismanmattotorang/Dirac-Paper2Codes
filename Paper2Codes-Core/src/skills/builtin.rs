//! Engine-shipped built-in domain skills.
//!
//! Each skill encodes a compact domain expertise primer, retrieval keywords,
//! supported languages, and idiomatic library recommendations. Users can
//! override any of these by dropping a `<id>.toml` file in their skills
//! directory (see [`super::SkillRegistry::load_user_dir`]).

use std::collections::HashMap;

use super::Skill;

fn libs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(lang, ls)| {
            (
                lang.to_string(),
                ls.iter().map(|s| s.to_string()).collect(),
            )
        })
        .collect()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

/// The full set of built-in domain skills.
pub fn builtin_skills() -> Vec<Skill> {
    vec![
        Skill {
            id: "computational-finance".into(),
            name: "Computational Finance".into(),
            version: "1.0.0".into(),
            description: "Quantitative finance: pricing, risk, portfolio optimisation, time series.".into(),
            domain: Some("Computational Finance".into()),
            keywords: s(&[
                "option pricing", "black-scholes", "monte carlo", "stochastic", "volatility",
                "portfolio", "risk", "var", "derivative", "interest rate", "time series", "garch",
            ]),
            languages: s(&["python", "cpp", "rust"]),
            libraries: libs(&[
                ("python", &["numpy", "pandas", "scipy", "QuantLib", "statsmodels", "cvxpy"]),
                ("cpp", &["QuantLib", "Eigen", "Boost"]),
                ("rust", &["ndarray", "statrs", "nalgebra"]),
            ]),
            instructions: "Implement quantitative-finance methods with numerical care: use \
                vectorised array operations, prefer closed-form solutions where the paper gives \
                them and Monte-Carlo/finite-difference schemes otherwise, and keep units and \
                day-count/discounting conventions explicit. Validate against analytical limits.".into(),
            verification_hints: s(&[
                "no-arbitrage / put-call parity holds where applicable",
                "Monte-Carlo estimates converge to closed-form values within tolerance",
                "risk metrics (VaR, Greeks) are dimensionally consistent",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-physics".into(),
            name: "Computational Physics".into(),
            version: "1.0.0".into(),
            description: "Numerical simulation of physical systems: ODE/PDE solvers, dynamics, fields.".into(),
            domain: Some("Computational Physics".into()),
            keywords: s(&[
                "simulation", "ode", "pde", "hamiltonian", "lagrangian", "finite difference",
                "finite element", "integrator", "conservation", "energy", "molecular dynamics",
            ]),
            languages: s(&["python", "cpp", "rust"]),
            libraries: libs(&[
                ("python", &["numpy", "scipy", "sympy", "matplotlib"]),
                ("cpp", &["Eigen", "Boost.Odeint", "GSL"]),
                ("rust", &["ndarray", "nalgebra"]),
            ]),
            instructions: "Translate physical models into numerically stable solvers. Choose \
                integrators appropriate to the dynamics (symplectic for Hamiltonian systems, \
                stiff solvers for stiff ODEs), state initial/boundary conditions explicitly, and \
                track conserved quantities to guard against numerical drift.".into(),
            verification_hints: s(&[
                "conserved quantities (energy, momentum) are preserved within tolerance",
                "solver is stable for the chosen step size (CFL where applicable)",
                "dimensional analysis of equations is consistent",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-chemistry".into(),
            name: "Computational Chemistry".into(),
            version: "1.0.0".into(),
            description: "Molecular modelling: electronic structure, force fields, cheminformatics.".into(),
            domain: Some("Computational Chemistry".into()),
            keywords: s(&[
                "molecule", "molecular", "dft", "hartree-fock", "force field", "smiles",
                "quantum chemistry", "basis set", "energy minimisation", "docking", "reaction",
            ]),
            languages: s(&["python", "cpp"]),
            libraries: libs(&[
                ("python", &["rdkit", "ase", "pyscf", "openmm", "numpy", "scipy"]),
                ("cpp", &["Eigen", "OpenBabel"]),
            ]),
            instructions: "Implement chemistry methods with correct handling of molecular \
                representations (SMILES/3D coordinates), units (Hartree/eV/kcal·mol⁻¹), and \
                basis sets. Prefer established toolkits (RDKit, ASE, PySCF) for parsing, geometry, \
                and electronic-structure primitives rather than re-deriving them.".into(),
            verification_hints: s(&[
                "molecular geometries and valences are chemically valid",
                "energies are reported in consistent, stated units",
                "optimisation reaches a stationary point (gradient norm below tolerance)",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-biology-bioinformatics".into(),
            name: "Computational Biology & Bioinformatics".into(),
            version: "1.0.0".into(),
            description: "Sequence analysis, alignment, phylogenetics, structural and systems biology.".into(),
            domain: Some("Computational Biology".into()),
            keywords: s(&[
                "sequence", "alignment", "blast", "phylogenetic", "protein", "dna", "rna",
                "fasta", "fastq", "msa", "hidden markov", "structure prediction",
            ]),
            languages: s(&["python", "rust"]),
            libraries: libs(&[
                ("python", &["biopython", "numpy", "pandas", "scikit-bio", "pysam"]),
                ("rust", &["rust-bio", "ndarray"]),
            ]),
            instructions: "Implement biological-sequence and structure methods using standard \
                file formats (FASTA/FASTQ/PDB) and established algorithms (Needleman-Wunsch, \
                Smith-Waterman, HMMs). Be explicit about alphabets, scoring matrices, and \
                gap penalties, and handle ambiguous/missing residues gracefully.".into(),
            verification_hints: s(&[
                "alignment scores match the stated scoring scheme on small examples",
                "sequence parsing round-trips standard formats",
                "biological constraints (e.g. valid nucleotide/amino-acid alphabets) hold",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-genomics".into(),
            name: "Computational Genomics".into(),
            version: "1.0.0".into(),
            description: "Genome assembly, variant calling, read mapping, and large-scale genomics.".into(),
            domain: Some("Computational Genomics".into()),
            keywords: s(&[
                "genome", "variant", "vcf", "bam", "sam", "read mapping", "assembly", "gwas",
                "snp", "sequencing", "coverage", "annotation",
            ]),
            languages: s(&["python", "rust"]),
            libraries: libs(&[
                ("python", &["pysam", "biopython", "numpy", "pandas", "scikit-allel"]),
                ("rust", &["rust-bio", "noodles", "ndarray"]),
            ]),
            instructions: "Implement genomics pipelines around standard formats (FASTQ/SAM/BAM/VCF) \
                and coordinate systems (0- vs 1-based). Stream large files rather than loading them \
                wholesale, document reference-genome assumptions, and keep variant representations \
                normalised (left-aligned, parsimonious).".into(),
            verification_hints: s(&[
                "coordinate conventions (0/1-based) are applied consistently",
                "VCF/BAM records round-trip and remain spec-valid",
                "variant normalisation is idempotent",
            ]),
            builtin: true,
        },
        Skill {
            id: "quantum-computation".into(),
            name: "Quantum Computation".into(),
            version: "1.0.0".into(),
            description: "Quantum circuits, algorithms, and simulation (gate-model and variational).".into(),
            domain: Some("Quantum Computing".into()),
            keywords: s(&[
                "qubit", "quantum circuit", "gate", "entanglement", "superposition", "vqe",
                "qaoa", "grover", "shor", "hamiltonian", "ansatz", "measurement",
            ]),
            languages: s(&["python"]),
            libraries: libs(&[
                ("python", &["qiskit", "cirq", "pennylane", "qutip", "numpy"]),
            ]),
            instructions: "Implement quantum algorithms as circuits with explicit qubit registers, \
                gate sequences, and measurement. Track qubit ordering/endianness, keep state-vector \
                or density-matrix conventions consistent, and prefer established SDKs (Qiskit, Cirq, \
                PennyLane) for simulation and transpilation.".into(),
            verification_hints: s(&[
                "quantum states remain normalised (probabilities sum to 1)",
                "unitary gates preserve the norm; circuits are reversible where claimed",
                "known algorithms reproduce expected outcomes on small instances",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-fluid-dynamics".into(),
            name: "Computational Fluid Dynamics".into(),
            version: "1.0.0".into(),
            description: "Fluid flow simulation: Navier-Stokes solvers, meshing, turbulence models.".into(),
            domain: Some("Computational Fluid Dynamics".into()),
            keywords: s(&[
                "navier-stokes", "fluid", "flow", "turbulence", "reynolds", "mesh", "finite volume",
                "cfd", "boundary layer", "incompressible", "advection", "diffusion",
            ]),
            languages: s(&["python", "cpp"]),
            libraries: libs(&[
                ("python", &["numpy", "scipy", "fenics", "fipy", "matplotlib"]),
                ("cpp", &["Eigen", "OpenFOAM", "deal.II"]),
            ]),
            instructions: "Implement CFD solvers with explicit discretisation (finite difference/\
                volume/element), grid definition, and boundary conditions. Respect stability limits \
                (CFL condition), state the turbulence/closure model, and verify against canonical \
                benchmarks (lid-driven cavity, Poiseuille flow) where possible.".into(),
            verification_hints: s(&[
                "CFL / stability condition is satisfied for the chosen time step",
                "mass is conserved (divergence-free velocity for incompressible flow)",
                "results match canonical benchmark cases within tolerance",
            ]),
            builtin: true,
        },
        Skill {
            id: "computational-supply-chain".into(),
            name: "Computational Supply Chain Management".into(),
            version: "1.0.0".into(),
            description: "Optimisation and simulation of supply chains: inventory, routing, scheduling.".into(),
            domain: Some("Supply Chain Algorithms".into()),
            keywords: s(&[
                "supply chain", "inventory", "logistics", "routing", "vehicle routing", "scheduling",
                "linear programming", "mixed integer", "optimisation", "demand forecasting", "network flow",
            ]),
            languages: s(&["python", "rust"]),
            libraries: libs(&[
                ("python", &["pulp", "ortools", "pyomo", "networkx", "simpy", "pandas"]),
                ("rust", &["good_lp", "petgraph", "ndarray"]),
            ]),
            instructions: "Model supply-chain problems as explicit optimisation (LP/MIP) or \
                discrete-event simulations. Define decision variables, objective, and constraints \
                precisely; choose solid solvers (OR-Tools, PuLP/Pyomo); and validate feasibility and \
                optimality on small instances before scaling.".into(),
            verification_hints: s(&[
                "solutions satisfy all constraints (feasibility)",
                "objective value matches a hand-computed optimum on small instances",
                "flow conservation holds at every network node",
            ]),
            builtin: true,
        },
    ]
}
