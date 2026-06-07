use crate::config::Config;
use crate::error::recovery::ErrorRecovery;
use crate::error::{LLMError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LLMProvider {
    OpenRouter { model: String },
    OpenAI { model: String },
    Anthropic { model: String },
    XAI { model: String },
}

#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse>;
    async fn stream(
        &self,
        request: crate::llm::LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>>;
    fn provider(&self) -> LLMProvider;
    fn model(&self) -> String;
}

pub struct OpenRouterClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    timeout_seconds: u64,
    max_retries: u32,
    retry_delay_ms: u64,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self::with_config(api_key, model, None)
    }

    pub fn with_config(api_key: String, model: String, config: Option<&Config>) -> Self {
        let timeout_seconds = config.map(|c| c.llm.timeout_seconds).unwrap_or(300);
        let max_retries = config.map(|c| c.llm.max_retries).unwrap_or(3);
        let retry_delay_ms = config.map(|c| c.llm.retry_delay_ms).unwrap_or(1000);

        // Create HTTP client with connection pooling and timeouts
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_while_idle(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
            client,
            model,
            timeout_seconds,
            max_retries,
            retry_delay_ms,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    async fn complete_with_retry(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let timeout_duration = Duration::from_secs(self.timeout_seconds);

        ErrorRecovery::retry_with_backoff(
            || {
                let url = url.clone();
                let api_key = api_key.clone();
                let client = client.clone();
                let request = request.clone();
                let timeout_duration = timeout_duration;

                Box::pin(async move {
                    let response = timeout(
                        timeout_duration,
                        Self::make_request(&client, &url, &api_key, &request),
                    )
                    .await
                    .map_err(|_| LLMError::Timeout)?;

                    response
                })
            },
            self.max_retries,
            self.retry_delay_ms,
        )
        .await
    }

    async fn make_request(
        client: &reqwest::Client,
        url: &str,
        api_key: &str,
        request: &crate::llm::LLMRequest,
    ) -> Result<LLMResponse> {
        #[derive(Serialize)]
        struct RequestBody {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            stream: bool,
        }

        let body = RequestBody {
            model: request.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct Usage {
            total_tokens: Option<u32>,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            choices: Vec<Choice>,
            model: String,
            usage: Option<Usage>,
        }

        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LLMError::Timeout
                } else {
                    LLMError::Network(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LLMError::RateLimit.into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(LLMError::Authentication("Invalid API key".to_string()).into());
            }
            return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
        }

        let response_body: ResponseBody = response
            .json()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

        let choice = response_body
            .choices
            .first()
            .ok_or_else(|| LLMError::InvalidResponse("No choices in response".to_string()))?;

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            model: response_body.model,
            tokens_used: response_body.usage.and_then(|u| u.total_tokens),
            finish_reason: choice.finish_reason.clone(),
        })
    }
}

#[async_trait]
impl LLMClient for OpenRouterClient {
    async fn complete(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        // Validate request size to prevent excessive token usage
        let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
        const MAX_REQUEST_SIZE: usize = 1_000_000; // ~250k tokens at 4 chars/token
        if total_chars > MAX_REQUEST_SIZE {
            return Err(LLMError::InvalidRequest(format!(
                "Request too large: {} characters (max: {})",
                total_chars, MAX_REQUEST_SIZE
            ))
            .into());
        }

        self.complete_with_retry(request).await
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::OpenRouter {
            model: self.model.clone(),
        }
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    async fn stream(
        &self,
        request: crate::llm::LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>> {
        // Streaming not yet implemented for OpenRouter
        // Fallback to non-streaming
        let response = self.complete(request).await?;
        let content = response.content;
        let stream: Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin> =
            Box::new(futures::stream::iter(vec![Ok(content)]));
        Ok(stream)
    }
}

pub struct XAIClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    max_retries: u32,
    retry_delay_ms: u64,
}

impl XAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        // Create HTTP client with connection pooling and timeouts
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_while_idle(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            base_url: "https://api.x.ai/v1".to_string(),
            client,
            model,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[async_trait]
impl LLMClient for XAIClient {
    async fn complete(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        // Validate request size
        let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
        const MAX_REQUEST_SIZE: usize = 1_000_000;
        if total_chars > MAX_REQUEST_SIZE {
            return Err(LLMError::InvalidRequest(format!(
                "Request too large: {} characters (max: {})",
                total_chars, MAX_REQUEST_SIZE
            ))
            .into());
        }

        let url = format!("{}/chat/completions", self.base_url);

        #[derive(Serialize)]
        struct RequestBody {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
        }

        let body = RequestBody {
            model: request.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct Usage {
            total_tokens: Option<u32>,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            choices: Vec<Choice>,
            model: String,
            usage: Option<Usage>,
        }

        // Retry logic with exponential backoff
        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            let response = match timeout(
                Duration::from_secs(300),
                self.client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send(),
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    last_error = Some(if e.is_timeout() {
                        LLMError::Timeout
                    } else {
                        LLMError::Network(e)
                    });
                    if attempt < self.max_retries {
                        let delay = self.retry_delay_ms * (1 << attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(last_error.unwrap().into());
                }
                Err(_) => {
                    last_error = Some(LLMError::Timeout);
                    if attempt < self.max_retries {
                        let delay = self.retry_delay_ms * (1 << attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(LLMError::Timeout.into());
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    last_error = Some(LLMError::RateLimit);
                    if attempt < self.max_retries {
                        let delay = self.retry_delay_ms * (1 << attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(LLMError::RateLimit.into());
                }
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(LLMError::Authentication("Invalid API key".to_string()).into());
                }
                return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
            }

            let response_body: ResponseBody = response.json().await.map_err(|e| {
                LLMError::InvalidResponse(format!("Failed to parse response: {}", e))
            })?;

            let choice = response_body
                .choices
                .first()
                .ok_or_else(|| LLMError::InvalidResponse("No choices in response".to_string()))?;

            return Ok(LLMResponse {
                content: choice.message.content.clone(),
                model: response_body.model,
                tokens_used: response_body.usage.and_then(|u| u.total_tokens),
                finish_reason: choice.finish_reason.clone(),
            });
        }

        Err(last_error.unwrap_or(LLMError::Timeout).into())
    }

    async fn stream(
        &self,
        request: crate::llm::LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>> {
        // Streaming not yet implemented for xAI
        // Fallback to non-streaming
        let response = self.complete(request).await?;
        let content = response.content;
        let stream: Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin> =
            Box::new(futures::stream::iter(vec![Ok(content)]));
        Ok(stream)
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::XAI {
            model: self.model.clone(),
        }
    }

    fn model(&self) -> String {
        self.model.clone()
    }
}

pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    max_retries: u32,
    retry_delay_ms: u64,
}

impl OpenAIClient {
    pub fn new(api_key: String, model: String) -> Self {
        // Create HTTP client with connection pooling and timeouts
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_while_idle(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client,
            model,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

impl OpenAIClient {
    async fn complete_with_retry(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let timeout_duration = Duration::from_secs(300);

        ErrorRecovery::retry_with_backoff(
            || {
                let url = url.clone();
                let api_key = api_key.clone();
                let client = client.clone();
                let request = request.clone();
                let timeout_duration = timeout_duration;

                Box::pin(async move {
                    let response = timeout(
                        timeout_duration,
                        Self::make_request(&client, &url, &api_key, &request),
                    )
                    .await
                    .map_err(|_| LLMError::Timeout)?;

                    response
                })
            },
            self.max_retries,
            self.retry_delay_ms,
        )
        .await
    }

    async fn make_request(
        client: &reqwest::Client,
        url: &str,
        api_key: &str,
        request: &crate::llm::LLMRequest,
    ) -> Result<LLMResponse> {
        #[derive(Serialize)]
        struct RequestBody {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            stream: bool,
        }

        let body = RequestBody {
            model: request.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct Usage {
            total_tokens: Option<u32>,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            choices: Vec<Choice>,
            model: String,
            usage: Option<Usage>,
        }

        let response = client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LLMError::Timeout
                } else {
                    LLMError::Network(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LLMError::RateLimit.into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(LLMError::Authentication("Invalid API key".to_string()).into());
            }
            return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
        }

        let response_body: ResponseBody = response
            .json()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

        let choice = response_body
            .choices
            .first()
            .ok_or_else(|| LLMError::InvalidResponse("No choices in response".to_string()))?;

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            model: response_body.model,
            tokens_used: response_body.usage.and_then(|u| u.total_tokens),
            finish_reason: choice.finish_reason.clone(),
        })
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    async fn complete(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        // Validate request size to prevent excessive token usage
        let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
        const MAX_REQUEST_SIZE: usize = 1_000_000; // ~250k tokens at 4 chars/token
        if total_chars > MAX_REQUEST_SIZE {
            return Err(LLMError::InvalidRequest(format!(
                "Request too large: {} characters (max: {})",
                total_chars, MAX_REQUEST_SIZE
            ))
            .into());
        }

        self.complete_with_retry(request).await
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::OpenAI {
            model: self.model.clone(),
        }
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    async fn stream(
        &self,
        request: crate::llm::LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>> {
        // Streaming not yet implemented for OpenAI
        // Fallback to non-streaming
        let response = self.complete(request).await?;
        let content = response.content;
        let stream: Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin> =
            Box::new(futures::stream::iter(vec![Ok(content)]));
        Ok(stream)
    }
}

pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    model: String,
    max_retries: u32,
    retry_delay_ms: u64,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        // Create HTTP client with connection pooling and timeouts
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_while_idle(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client,
            model,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

impl AnthropicClient {
    async fn complete_with_retry(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        let url = format!("{}/messages", self.base_url);
        let api_key = self.api_key.clone();
        let client = self.client.clone();
        let timeout_duration = Duration::from_secs(300);

        ErrorRecovery::retry_with_backoff(
            || {
                let url = url.clone();
                let api_key = api_key.clone();
                let client = client.clone();
                let request = request.clone();
                let timeout_duration = timeout_duration;

                Box::pin(async move {
                    let response = timeout(
                        timeout_duration,
                        Self::make_request(&client, &url, &api_key, &request),
                    )
                    .await
                    .map_err(|_| LLMError::Timeout)?;

                    response
                })
            },
            self.max_retries,
            self.retry_delay_ms,
        )
        .await
    }

    async fn make_request(
        client: &reqwest::Client,
        url: &str,
        api_key: &str,
        request: &crate::llm::LLMRequest,
    ) -> Result<LLMResponse> {
        #[derive(Serialize)]
        struct RequestBody {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
        }

        let body = RequestBody {
            model: request.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens.or(Some(4096)),
        };

        #[derive(Deserialize)]
        struct Content {
            #[serde(rename = "type")]
            _content_type: String,
            text: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            input_tokens: u32,
            output_tokens: u32,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            content: Vec<Content>,
            model: String,
            usage: Usage,
            stop_reason: Option<String>,
        }

        let response = client
            .post(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LLMError::Timeout
                } else {
                    LLMError::Network(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LLMError::RateLimit.into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(LLMError::Authentication("Invalid API key".to_string()).into());
            }
            return Err(LLMError::RequestFailed(format!("HTTP {}: {}", status, text)).into());
        }

        let response_body: ResponseBody = response
            .json()
            .await
            .map_err(|e| LLMError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

        let content = response_body
            .content
            .first()
            .ok_or_else(|| LLMError::InvalidResponse("No content in response".to_string()))?;

        Ok(LLMResponse {
            content: content.text.clone(),
            model: response_body.model,
            tokens_used: Some(response_body.usage.input_tokens + response_body.usage.output_tokens),
            finish_reason: response_body.stop_reason,
        })
    }
}

#[async_trait]
impl LLMClient for AnthropicClient {
    async fn complete(&self, request: crate::llm::LLMRequest) -> Result<LLMResponse> {
        // Validate request size to prevent excessive token usage
        let total_chars: usize = request.messages.iter().map(|m| m.content.len()).sum();
        const MAX_REQUEST_SIZE: usize = 1_000_000; // ~250k tokens at 4 chars/token
        if total_chars > MAX_REQUEST_SIZE {
            return Err(LLMError::InvalidRequest(format!(
                "Request too large: {} characters (max: {})",
                total_chars, MAX_REQUEST_SIZE
            ))
            .into());
        }

        self.complete_with_retry(request).await
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::Anthropic {
            model: self.model.clone(),
        }
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    async fn stream(
        &self,
        request: crate::llm::LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>> {
        // Streaming not yet implemented for Anthropic
        // Fallback to non-streaming
        let response = self.complete(request).await?;
        let content = response.content;
        let stream: Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin> =
            Box::new(futures::stream::iter(vec![Ok(content)]));
        Ok(stream)
    }
}
