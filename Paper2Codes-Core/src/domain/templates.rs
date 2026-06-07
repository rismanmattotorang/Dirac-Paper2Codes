//! Domain-specific code templates and generation helpers

use crate::domain::ComputationalDomain;
use std::collections::HashMap;

/// Domain-specific code templates
pub struct DomainTemplates;

impl DomainTemplates {
    /// Get code template for a domain and language
    pub fn get_template(domain: ComputationalDomain, language: &str) -> Option<&'static str> {
        let templates = domain.code_templates();
        templates.get(language).copied()
    }

    /// Get domain-specific prompt guidance
    pub fn get_prompt_guidance(domain: ComputationalDomain) -> String {
        let languages = domain.preferred_languages().join(", ");
        let frameworks = domain.preferred_frameworks().join(", ");

        format!(
            "This paper is in the domain of {}.\n\
            Preferred programming languages: {}.\n\
            Recommended frameworks/libraries: {}.\n\
            Generate production-quality code following best practices for this domain.",
            domain, languages, frameworks
        )
    }
}

impl ComputationalDomain {
    /// Get code templates for this domain
    pub fn code_templates(&self) -> HashMap<&'static str, &'static str> {
        let mut templates = HashMap::new();
        match self {
            Self::NumericalComputing => {
                templates.insert(
                    "python",
                    "import numpy as np\nimport scipy as sp\n\ndef solve_numerical_problem(data):\n    # Implementation\n    pass",
                );
                templates.insert(
                    "rust",
                    "use ndarray::prelude::*;\n\nfn solve_numerical_problem(data: &Array2<f64>) -> Array1<f64> {\n    // Implementation\n    Array1::zeros(data.nrows())\n}",
                );
            }
            Self::DeepLearning => {
                templates.insert(
                    "python",
                    "import torch\nimport torch.nn as nn\n\nclass NeuralNetwork(nn.Module):\n    def __init__(self):\n        super().__init__()\n        # Define layers\n\n    def forward(self, x):\n        # Forward pass\n        return x",
                );
            }
            Self::QuantumComputing => {
                templates.insert(
                    "python",
                    "import qiskit\nfrom qiskit import QuantumCircuit\n\ndef create_quantum_circuit():\n    qc = QuantumCircuit(2, 2)\n    return qc",
                );
            }
            _ => {}
        }
        templates
    }
}
