//! Domain detection for research papers
//!
//! Automatically identifies computational domains in research papers using
//! hybrid rule-based and LLM-based detection methods.

use crate::error::Result;
use crate::llm::client::{LLMClient, Message, MessageRole};
use crate::llm::LLMRequest;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Computational domains that can be detected in research papers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputationalDomain {
    /// Numerical computing and simulations
    NumericalComputing,
    /// Chip design and hardware optimization
    ChipDesign,
    /// Bioinformatics and functional genomics
    Bioinformatics,
    /// Quantum computing algorithms
    QuantumComputing,
    /// Digital twin simulations
    DigitalTwin,
    /// Classical machine learning models
    ClassicalML,
    /// Deep learning models and neural networks
    DeepLearning,
    /// Transformer-based generative models
    Transformers,
    /// Computational physics
    ComputationalPhysics,
    /// Computational biology
    ComputationalBiology,
    /// Computational finance and quantitative finance
    ComputationalFinance,
    /// Supply chain algorithms and optimization
    SupplyChain,
    /// Logistics and distribution algorithms
    Logistics,
    /// General or undetected domain
    General,
}

impl std::fmt::Display for ComputationalDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NumericalComputing => write!(f, "Numerical Computing"),
            Self::ChipDesign => write!(f, "Chip Design and Optimization"),
            Self::Bioinformatics => write!(f, "Bioinformatics and Functional Genomics"),
            Self::QuantumComputing => write!(f, "Quantum Computing"),
            Self::DigitalTwin => write!(f, "Digital Twin Simulations"),
            Self::ClassicalML => write!(f, "Classical Machine Learning"),
            Self::DeepLearning => write!(f, "Deep Learning"),
            Self::Transformers => write!(f, "Transformer-based Models"),
            Self::ComputationalPhysics => write!(f, "Computational Physics"),
            Self::ComputationalBiology => write!(f, "Computational Biology"),
            Self::ComputationalFinance => write!(f, "Computational Finance"),
            Self::SupplyChain => write!(f, "Supply Chain Algorithms"),
            Self::Logistics => write!(f, "Logistics and Distribution"),
            Self::General => write!(f, "General Computing"),
        }
    }
}

impl ComputationalDomain {
    /// Get the preferred programming languages for this domain
    pub fn preferred_languages(&self) -> Vec<&'static str> {
        match self {
            Self::NumericalComputing => vec!["python", "c++", "rust"],
            Self::ChipDesign => vec!["c", "c++", "rust"],
            Self::Bioinformatics => vec!["nextflow", "python", "rust"],
            Self::QuantumComputing => vec!["python", "rust"],
            Self::DigitalTwin => vec!["python", "rust", "c++"],
            Self::ClassicalML => vec!["python", "rust"],
            Self::DeepLearning => vec!["python", "rust"],
            Self::Transformers => vec!["python", "rust"],
            Self::ComputationalPhysics => vec!["c++", "python", "rust"],
            Self::ComputationalBiology => vec!["c++", "python", "rust"],
            Self::ComputationalFinance => vec!["c++", "python", "rust"],
            Self::SupplyChain => vec!["c++", "python", "rust"],
            Self::Logistics => vec!["c++", "python", "rust"],
            Self::General => vec!["python", "rust", "javascript"],
        }
    }

    /// Get the preferred frameworks/libraries for this domain
    pub fn preferred_frameworks(&self) -> Vec<&'static str> {
        match self {
            Self::NumericalComputing => vec!["numpy", "scipy", "ndarray"],
            Self::ChipDesign => vec!["verilator", "chisel", "verilog", "systemverilog"],
            Self::Bioinformatics => vec!["nextflow", "biopython", "rust-bio"],
            Self::QuantumComputing => vec!["qiskit", "cirq", "qutip", "pennylane"],
            Self::DigitalTwin => vec!["simpy", "mesa", "anylogic", "rust-sim"],
            Self::ClassicalML => vec!["scikit-learn", "linfa", "xgboost"],
            Self::DeepLearning => vec!["pytorch", "pytorch-lightning", "tch-rs"],
            Self::Transformers => vec!["transformers", "langchain", "tokenizers"],
            Self::ComputationalPhysics => vec!["numpy", "scipy", "lammps", "opensim"],
            Self::ComputationalBiology => vec!["biopython", "rust-bio", "bioconductor"],
            Self::ComputationalFinance => vec!["quantlib", "numpy-financial", "pandas"],
            Self::SupplyChain => vec!["pyomo", "ortools", "simpy"],
            Self::Logistics => vec!["ortools", "pyomo", "networkx"],
            Self::General => vec!["pandas", "numpy", "ndarray"],
        }
    }
}

/// Domain detector to identify computational domains in research papers
pub struct DomainDetector {
    domain_keywords: HashMap<ComputationalDomain, Vec<String>>,
    regex_patterns: HashMap<ComputationalDomain, Vec<Regex>>,
    llm_client: Option<Arc<dyn LLMClient>>,
    min_confidence: f64,
}

lazy_static! {
    static ref DOMAIN_CACHE: tokio::sync::Mutex<HashMap<String, ComputationalDomain>> =
        tokio::sync::Mutex::new(HashMap::new());
}

impl Default for DomainDetector {
    fn default() -> Self {
        let mut detector = Self::new();
        detector.initialize_domain_keywords();
        detector.initialize_domain_patterns();
        detector
    }
}

impl DomainDetector {
    /// Create a new domain detector
    pub fn new() -> Self {
        Self {
            domain_keywords: HashMap::new(),
            regex_patterns: HashMap::new(),
            llm_client: None,
            min_confidence: 0.6,
        }
    }

    /// Create a new domain detector with LLM client for enhanced detection
    pub fn with_llm(llm_client: Arc<dyn LLMClient>, min_confidence: f64) -> Self {
        let mut detector = Self {
            domain_keywords: HashMap::new(),
            regex_patterns: HashMap::new(),
            llm_client: Some(llm_client),
            min_confidence,
        };
        detector.initialize_domain_keywords();
        detector.initialize_domain_patterns();
        detector
    }

    /// Initialize domain-specific keywords for detection
    fn initialize_domain_keywords(&mut self) {
        // Numerical Computing
        self.domain_keywords.insert(
            ComputationalDomain::NumericalComputing,
            vec![
                "numerical method".to_string(),
                "numerical analysis".to_string(),
                "numerical simulation".to_string(),
                "finite element".to_string(),
                "finite difference".to_string(),
                "differential equation".to_string(),
                "numerical integration".to_string(),
                "discretization".to_string(),
            ],
        );

        // Deep Learning
        self.domain_keywords.insert(
            ComputationalDomain::DeepLearning,
            vec![
                "deep learning".to_string(),
                "neural network".to_string(),
                "cnn".to_string(),
                "convolutional neural".to_string(),
                "rnn".to_string(),
                "lstm".to_string(),
                "backpropagation".to_string(),
            ],
        );

        // Transformers
        self.domain_keywords.insert(
            ComputationalDomain::Transformers,
            vec![
                "transformer model".to_string(),
                "attention mechanism".to_string(),
                "self-attention".to_string(),
                "bert".to_string(),
                "gpt".to_string(),
                "language model".to_string(),
                "llm".to_string(),
            ],
        );

        // Quantum Computing
        self.domain_keywords.insert(
            ComputationalDomain::QuantumComputing,
            vec![
                "quantum computing".to_string(),
                "quantum algorithm".to_string(),
                "qubit".to_string(),
                "quantum circuit".to_string(),
            ],
        );

        // Add more domains as needed...
    }

    /// Initialize domain-specific regex patterns for detection
    fn initialize_domain_patterns(&mut self) {
        let numerical_patterns =
            vec![
                Regex::new(r"(?i)\b(numerical method|finite element|finite difference)\b").unwrap(),
            ];
        self.regex_patterns
            .insert(ComputationalDomain::NumericalComputing, numerical_patterns);

        let dl_patterns =
            vec![Regex::new(r"(?i)\b(neural network|deep learning|CNN|RNN|LSTM)\b").unwrap()];
        self.regex_patterns
            .insert(ComputationalDomain::DeepLearning, dl_patterns);
    }

    /// Detect domain using hybrid approach (rules + optional LLM)
    pub async fn detect_domain(&self, text_chunks: &[String]) -> Result<ComputationalDomain> {
        let combined_text = text_chunks.join(" ");

        // Check cache first
        let text_hash = compute_hash(&combined_text).to_string();
        {
            let cache = DOMAIN_CACHE.lock().await;
            if let Some(cached) = cache.get(&text_hash) {
                debug!("Domain detected from cache: {:?}", cached);
                return Ok(*cached);
            }
        }

        // Try rule-based detection first
        let rule_based_domain = self.detect_domain_with_rules(&combined_text);

        // If LLM client is available and rule-based is uncertain, use LLM
        if let Some(llm_client) = &self.llm_client {
            if rule_based_domain == ComputationalDomain::General {
                match self
                    .detect_domain_with_llm(text_chunks, llm_client.as_ref())
                    .await
                {
                    Ok(llm_domain) => {
                        if llm_domain != ComputationalDomain::General {
                            // Cache and return LLM result
                            let mut cache = DOMAIN_CACHE.lock().await;
                            cache.insert(text_hash, llm_domain);
                            return Ok(llm_domain);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "LLM domain detection failed: {}, falling back to rule-based",
                            e
                        );
                    }
                }
            }
        }

        // Cache and return rule-based result
        let mut cache = DOMAIN_CACHE.lock().await;
        cache.insert(text_hash, rule_based_domain);
        Ok(rule_based_domain)
    }

    /// Rule-based domain detection
    fn detect_domain_with_rules(&self, text: &str) -> ComputationalDomain {
        let text_lower = text.to_lowercase();
        let mut domain_scores: HashMap<ComputationalDomain, usize> = HashMap::new();

        // Score based on keywords
        for (domain, keywords) in &self.domain_keywords {
            for keyword in keywords {
                let count = text_lower.matches(&keyword.to_lowercase()).count();
                *domain_scores.entry(*domain).or_insert(0) += count;
            }
        }

        // Score based on regex patterns
        for (domain, patterns) in &self.regex_patterns {
            for pattern in patterns {
                let count = pattern.find_iter(&text_lower).count();
                *domain_scores.entry(*domain).or_insert(0) += count * 2;
            }
        }

        // Find domain with highest score
        let (detected_domain, max_score) = domain_scores
            .into_iter()
            .max_by_key(|(_, score)| *score)
            .unwrap_or((ComputationalDomain::General, 0));

        // Check confidence threshold
        let confidence = max_score as f64 / 20.0;
        if confidence < self.min_confidence {
            ComputationalDomain::General
        } else {
            debug!(
                "Detected domain: {:?} with confidence: {:.2}",
                detected_domain, confidence
            );
            detected_domain
        }
    }

    /// Detect domain using LLM
    async fn detect_domain_with_llm(
        &self,
        text_chunks: &[String],
        llm_client: &dyn LLMClient,
    ) -> Result<ComputationalDomain> {
        // Create summary from first few chunks
        let mut summary = text_chunks
            .iter()
            .take(3)
            .fold(String::new(), |mut acc, chunk| {
                acc.push_str(chunk);
                acc.push_str("\n\n");
                acc
            });

        if summary.len() > 3000 {
            summary = summary[..3000].to_string();
        }

        let prompt = format!(
            "You are an expert at analyzing research papers and identifying their computational domain.

Read the following excerpt from a research paper and determine its primary computational domain from this list:
- Numerical Computing
- Chip Design and Optimization
- Bioinformatics and Functional Genomics
- Quantum Computing
- Digital Twin Simulations
- Classical Machine Learning
- Deep Learning
- Transformer-based Models
- Computational Physics
- Computational Biology
- Computational Finance
- Supply Chain Algorithms
- Logistics and Distribution
- General Computing (if none of the above apply)

Paper excerpt:
```
{}
```

Analyze the terminology, methods, algorithms, and technologies mentioned in the paper.
Return ONLY the name of the computational domain that best matches this paper. Respond with exactly one domain name from the list above.",
            summary
        );

        let request = LLMRequest {
            model: llm_client.model(),
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content:
                        "You are an expert at identifying computational domains in research papers."
                            .to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: prompt,
                },
            ],
            temperature: 0.0,
            max_tokens: Some(50),
            stream: false,
        };

        let response = llm_client.complete(request).await?;
        self.parse_domain_from_llm_response(&response.content)
    }

    /// Parse domain from LLM response
    fn parse_domain_from_llm_response(&self, response: &str) -> Result<ComputationalDomain> {
        let response_lower = response.trim().to_lowercase();

        if response_lower.contains("numerical") {
            Ok(ComputationalDomain::NumericalComputing)
        } else if response_lower.contains("chip design") || response_lower.contains("hardware") {
            Ok(ComputationalDomain::ChipDesign)
        } else if response_lower.contains("bioinformatics") || response_lower.contains("genomics") {
            Ok(ComputationalDomain::Bioinformatics)
        } else if response_lower.contains("quantum") {
            Ok(ComputationalDomain::QuantumComputing)
        } else if response_lower.contains("digital twin") {
            Ok(ComputationalDomain::DigitalTwin)
        } else if response_lower.contains("classical machine learning") {
            Ok(ComputationalDomain::ClassicalML)
        } else if response_lower.contains("deep learning") {
            Ok(ComputationalDomain::DeepLearning)
        } else if response_lower.contains("transformer") {
            Ok(ComputationalDomain::Transformers)
        } else if response_lower.contains("computational physics") {
            Ok(ComputationalDomain::ComputationalPhysics)
        } else if response_lower.contains("computational biology") {
            Ok(ComputationalDomain::ComputationalBiology)
        } else if response_lower.contains("computational finance")
            || response_lower.contains("finance")
        {
            Ok(ComputationalDomain::ComputationalFinance)
        } else if response_lower.contains("supply chain") {
            Ok(ComputationalDomain::SupplyChain)
        } else if response_lower.contains("logistics") {
            Ok(ComputationalDomain::Logistics)
        } else {
            Ok(ComputationalDomain::General)
        }
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn compute_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
