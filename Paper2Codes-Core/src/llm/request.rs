use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<crate::llm::Message>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub model: String,
}

impl Default for LLMRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            temperature: 0.7,
            max_tokens: None,
            stream: false,
            model: String::new(),
        }
    }
}

impl LLMRequest {
    pub fn new(messages: Vec<crate::llm::Message>, model: String) -> Self {
        Self {
            messages,
            temperature: 0.7,
            max_tokens: None,
            stream: false,
            model,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}
