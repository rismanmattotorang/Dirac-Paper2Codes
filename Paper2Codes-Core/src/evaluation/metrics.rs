//! Benchmark metrics collection
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub overall_average_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub total_tokens: usize,
    pub api_calls: usize,
    pub tool_calls: usize,
}
