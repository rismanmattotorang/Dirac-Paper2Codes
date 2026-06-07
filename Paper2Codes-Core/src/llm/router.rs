use crate::config::Config;
use crate::error::{Paper2CodesError, Result};
use crate::llm::cache::{CacheKey, LLMCache};
use crate::llm::client::LLMClient;
use crate::llm::strategy::{
    LlmClientPreference, LlmStrategy, StrategyLLMClient, TaskType as StrategyTaskType,
};
use crate::llm::{LLMRequest, LLMResponse};
use crate::performance::rate_limiter::{RateLimitConfig, SmartRateLimiter};
use crate::types::Task;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct LLMRouter {
    clients: HashMap<String, Arc<dyn LLMClient>>,
    pub config: Config,
    cache: Arc<LLMCache>,
    enable_cache: bool,
    strategy: LlmStrategy,
    strategy_client: Option<StrategyLLMClient>,
    rate_limiter: Option<Arc<SmartRateLimiter>>,
}

impl LLMRouter {
    pub fn new(config: Config) -> Result<Self> {
        Self::with_cache(config, true)
    }

    pub fn with_cache(config: Config, enable_cache: bool) -> Result<Self> {
        let mut clients: HashMap<String, Arc<dyn LLMClient>> = HashMap::new();

        // Initialize clients based on config
        for (provider_name, provider_config) in &config.llm.providers {
            if !provider_config.enabled {
                continue;
            }

            let api_key = provider_config
                .api_key
                .clone()
                .or_else(|| std::env::var(format!("{}_API_KEY", provider_name.to_uppercase())).ok())
                .ok_or_else(|| {
                    crate::error::ConfigError::MissingField(format!(
                        "API key for {}",
                        provider_name
                    ))
                })?;

            match provider_name.as_str() {
                "openrouter" => {
                    if let Some(model) = provider_config.models.first() {
                        let mut client = crate::llm::client::OpenRouterClient::with_config(
                            api_key,
                            model.clone(),
                            Some(&config),
                        );
                        if let Some(base_url) = &provider_config.base_url {
                            client = client.with_base_url(base_url.clone());
                        }
                        clients.insert(provider_name.clone(), Arc::new(client));
                    }
                }
                "openai" => {
                    if let Some(model) = provider_config.models.first() {
                        let client: Arc<dyn LLMClient> = Arc::new(
                            crate::llm::client::OpenAIClient::new(api_key, model.clone()),
                        );
                        clients.insert(provider_name.clone(), client);
                    }
                }
                "anthropic" => {
                    if let Some(model) = provider_config.models.first() {
                        let client: Arc<dyn LLMClient> = Arc::new(
                            crate::llm::client::AnthropicClient::new(api_key, model.clone()),
                        );
                        clients.insert(provider_name.clone(), client);
                    }
                }
                "xai" | "x-ai" => {
                    if let Some(model) = provider_config.models.first() {
                        let client: Arc<dyn LLMClient> =
                            Arc::new(crate::llm::client::XAIClient::new(api_key, model.clone()));
                        clients.insert(provider_name.clone(), client);
                    }
                }
                _ => {}
            }
        }

        // Initialize cache (1000 entries, 1 hour TTL by default)
        let cache = Arc::new(LLMCache::new(1000, 3600));

        // Initialize strategy (default to adaptive)
        let strategy = LlmStrategy::default();

        // Create strategy client if we have both OpenAI and Anthropic
        let strategy_client = {
            let openai_client = clients.get("openai").cloned();
            let anthropic_client = clients.get("anthropic").cloned();

            if openai_client.is_some() || anthropic_client.is_some() {
                Some(StrategyLLMClient::new(
                    openai_client,
                    anthropic_client,
                    strategy.clone(),
                ))
            } else {
                None
            }
        };

        // Initialize rate limiter if configured
        let rate_limiter = if config.llm.max_retries > 0 {
            let rate_limit_config = RateLimitConfig {
                max_rpm: 60,              // Default: 60 requests per minute
                tokens_per_minute: 60000, // Default: 60k tokens per minute
                max_concurrent: 5,
                adaptive: true,
                initial_delay_ms: 100,
            };
            Some(Arc::new(SmartRateLimiter::new(rate_limit_config)))
        } else {
            None
        };

        Ok(Self {
            clients,
            config,
            cache,
            enable_cache,
            strategy,
            strategy_client,
            rate_limiter,
        })
    }

    /// Set routing strategy
    pub fn with_strategy(mut self, strategy: LlmStrategy) -> Self {
        self.strategy = strategy.clone();
        // Update strategy client if available
        if let Some(ref mut sc) = self.strategy_client {
            let openai = self.clients.get("openai").cloned();
            let anthropic = self.clients.get("anthropic").cloned();
            *sc = StrategyLLMClient::new(openai, anthropic, strategy);
        }
        self
    }

    pub fn cache(&self) -> Arc<LLMCache> {
        self.cache.clone()
    }

    /// Convert task type to strategy task type
    fn task_type_to_strategy_type(&self, task: &Task) -> StrategyTaskType {
        match task.task_type {
            crate::types::TaskType::Planning => StrategyTaskType::Planning,
            crate::types::TaskType::Analysis { .. } => StrategyTaskType::Analysis,
            crate::types::TaskType::Coding { .. } => StrategyTaskType::CodeGeneration,
            crate::types::TaskType::Verification { .. } => StrategyTaskType::Verification,
            crate::types::TaskType::Fix { .. } => StrategyTaskType::BugFixing,
        }
    }

    pub fn select_llm(&self, task: &Task) -> Result<Arc<dyn LLMClient>> {
        // Use strategy-based selection if strategy client is available
        if let Some(ref _strategy_client) = self.strategy_client {
            let strategy_task_type = self.task_type_to_strategy_type(task);
            let preference = self.strategy.get_preference(&strategy_task_type);

            match preference {
                LlmClientPreference::UseOpenAi => {
                    return self.clients.get("openai").cloned().ok_or_else(|| {
                        crate::error::Paper2CodesError::LLM(
                            crate::error::LLMError::ModelUnavailable(
                                "OpenAI client not available".to_string(),
                            ),
                        )
                    });
                }
                LlmClientPreference::UseClaude => {
                    return self.clients.get("anthropic").cloned().ok_or_else(|| {
                        crate::error::Paper2CodesError::LLM(
                            crate::error::LLMError::ModelUnavailable(
                                "Anthropic client not available".to_string(),
                            ),
                        )
                    });
                }
                LlmClientPreference::PreferOpenAi => {
                    if let Some(client) = self.clients.get("openai") {
                        return Ok(client.clone());
                    }
                    // Fallback to Anthropic
                    return self.clients.get("anthropic").cloned().ok_or_else(|| {
                        crate::error::Paper2CodesError::LLM(
                            crate::error::LLMError::ModelUnavailable(
                                "No LLM clients available".to_string(),
                            ),
                        )
                    });
                }
                LlmClientPreference::PreferClaude => {
                    if let Some(client) = self.clients.get("anthropic") {
                        return Ok(client.clone());
                    }
                    // Fallback to OpenAI
                    return self.clients.get("openai").cloned().ok_or_else(|| {
                        crate::error::Paper2CodesError::LLM(
                            crate::error::LLMError::ModelUnavailable(
                                "No LLM clients available".to_string(),
                            ),
                        )
                    });
                }
                LlmClientPreference::Both => {
                    // Will be handled in complete() method with strategy
                    // For now, prefer OpenAI
                    if let Some(client) = self.clients.get("openai") {
                        return Ok(client.clone());
                    }
                }
            }
        }

        // Fallback to default provider selection
        let model_name = match task.task_type {
            crate::types::TaskType::Planning => &self.config.agents.planning_model,
            crate::types::TaskType::Analysis { .. } => &self.config.agents.analysis_model,
            crate::types::TaskType::Coding { .. } => &self.config.agents.coding_model,
            crate::types::TaskType::Verification { .. } => &self.config.agents.verification_model,
            crate::types::TaskType::Fix { .. } => &self.config.agents.coding_model,
        };

        // Try to extract provider from model name
        let provider = if model_name.contains("gpt") || model_name.contains("openai") {
            "openai"
        } else if model_name.contains("claude") || model_name.contains("anthropic") {
            "anthropic"
        } else if model_name.contains("grok") || model_name.contains("xai") {
            "xai"
        } else {
            &self.config.llm.default_provider
        };

        self.clients.get(provider).cloned().ok_or_else(|| {
            crate::error::Paper2CodesError::LLM(crate::error::LLMError::ModelUnavailable(format!(
                "Provider {} not available",
                provider
            )))
        })
    }

    pub async fn complete(&self, request: LLMRequest, task: Option<&Task>) -> Result<LLMResponse> {
        // Check cache if enabled
        let cache_hit = if self.enable_cache {
            let cache_key =
                CacheKey::from_request(&request.model, &request.messages, request.temperature);

            // Try to get from cache
            if let Some(cached_response) = self.cache.get(&cache_key).await {
                debug!("Cache hit for model: {}", request.model);
                return Ok(cached_response);
            }
            false
        } else {
            false
        };

        // Estimate tokens for rate limiting (more accurate: ~3.5 chars per token for English)
        // Add overhead for message formatting (~10 tokens per message)
        let estimated_tokens = request
            .messages
            .iter()
            .map(|m| {
                let content_tokens = (m.content.len() as f32 / 3.5) as u32;
                content_tokens + 10 // Add overhead for message formatting
            })
            .sum::<u32>();

        // Acquire rate limit permit if rate limiter is enabled
        let _rate_limit_permit = if let Some(ref limiter) = self.rate_limiter {
            Some(limiter.acquire(Some(estimated_tokens)).await.map_err(|e| {
                warn!("Rate limit acquisition failed: {}", e);
                e
            })?)
        } else {
            None
        };

        // Execute request with error recovery
        // Use strategy-based routing if available and task is provided
        let mut response_result: Result<LLMResponse> =
            Err(Paper2CodesError::Validation("Initial attempt".to_string()));

        for attempt in 0..=self.config.llm.max_retries {
            response_result = {
                if let (Some(ref strategy_client), Some(t)) = (&self.strategy_client, task) {
                    let strategy_task_type = self.task_type_to_strategy_type(t);
                    let preference = self.strategy.get_preference(&strategy_task_type);

                    if matches!(preference, LlmClientPreference::Both) {
                        debug!("Using strategy-based routing with compare-and-merge");
                        strategy_client
                            .generate(request.clone(), strategy_task_type)
                            .await
                    } else {
                        let client = self.select_llm(t)?;
                        client.complete(request.clone()).await
                    }
                } else {
                    let client = if let Some(t) = task {
                        self.select_llm(t)?
                    } else {
                        self.clients
                            .get(&self.config.llm.default_provider)
                            .ok_or_else(|| {
                                Paper2CodesError::LLM(crate::error::LLMError::ModelUnavailable(
                                    "No default provider".to_string(),
                                ))
                            })?
                            .clone()
                    };
                    client.complete(request.clone()).await
                }
            };

            match response_result {
                Ok(_) => {
                    if attempt > 0 {
                        debug!("Request succeeded after {} retries", attempt);
                    }
                    break;
                }
                Err(ref e) => {
                    if !e.is_retryable() {
                        break;
                    }
                    if attempt < self.config.llm.max_retries {
                        let delay = self.config.llm.retry_delay_ms * (1 << attempt); // Exponential backoff
                        warn!(
                            "Retry attempt {}/{} after {}ms: {}",
                            attempt + 1,
                            self.config.llm.max_retries,
                            delay,
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        let response = response_result?;

        // Update rate limiter with actual tokens used
        if let Some(mut permit) = _rate_limit_permit {
            if let Some(tokens) = response.tokens_used {
                permit.set_tokens(tokens);
            }
            // Permit will be dropped here and tokens recorded
        }

        // Store in cache if enabled and it was a cache miss
        if self.enable_cache && !cache_hit {
            let cache_key =
                CacheKey::from_request(&request.model, &request.messages, request.temperature);
            self.cache.put(cache_key, response.clone()).await;
        }

        Ok(response)
    }

    pub async fn stream(
        &self,
        request: LLMRequest,
        task: Option<&Task>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>> {
        // Estimate tokens for rate limiting (same heuristic as complete)
        let estimated_tokens = request
            .messages
            .iter()
            .map(|m| {
                let content_tokens = (m.content.len() as f32 / 3.5) as u32;
                content_tokens + 10
            })
            .sum::<u32>();

        let _rate_limit_permit = if let Some(ref limiter) = self.rate_limiter {
            Some(limiter.acquire(Some(estimated_tokens)).await.map_err(|e| {
                warn!("Rate limit acquisition failed for streaming request: {}", e);
                e
            })?)
        } else {
            None
        };

        let client = if let Some(task) = task {
            self.select_llm(task)?
        } else {
            Self::select_client_for_task(&self.clients, &self.config, None)?
        };

        let mut stream_result: Result<
            Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>,
        > = Err(Paper2CodesError::Validation("Initial attempt".to_string()));

        for attempt in 0..=self.config.llm.max_retries {
            stream_result = client.stream(request.clone()).await;

            match stream_result {
                Ok(_) => {
                    if attempt > 0 {
                        debug!("Streaming request succeeded after {} retries", attempt);
                    }
                    break;
                }
                Err(ref e) => {
                    if !e.is_retryable() {
                        break;
                    }
                    if attempt < self.config.llm.max_retries {
                        let delay = self.config.llm.retry_delay_ms * (1 << attempt);
                        warn!(
                            "Retrying streaming request {}/{} after {}ms: {}",
                            attempt + 1,
                            self.config.llm.max_retries,
                            delay,
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        stream_result
    }

    /// Helper to select client for a task (extracted for reuse)
    fn select_client_for_task(
        clients: &HashMap<String, Arc<dyn LLMClient>>,
        config: &Config,
        task: Option<&Task>,
    ) -> Result<Arc<dyn LLMClient>> {
        if let Some(t) = task {
            let model_name = match t.task_type {
                crate::types::TaskType::Planning => &config.agents.planning_model,
                crate::types::TaskType::Analysis { .. } => &config.agents.analysis_model,
                crate::types::TaskType::Coding { .. } => &config.agents.coding_model,
                crate::types::TaskType::Verification { .. } => &config.agents.verification_model,
                crate::types::TaskType::Fix { .. } => &config.agents.coding_model,
            };

            let provider = if model_name.contains("gpt") || model_name.contains("openai") {
                "openai"
            } else if model_name.contains("claude") || model_name.contains("anthropic") {
                "anthropic"
            } else if model_name.contains("grok") || model_name.contains("xai") {
                "xai"
            } else {
                &config.llm.default_provider
            };

            clients.get(provider).cloned().ok_or_else(|| {
                Paper2CodesError::LLM(crate::error::LLMError::ModelUnavailable(format!(
                    "Provider {} not available",
                    provider
                )))
            })
        } else {
            clients
                .get(&config.llm.default_provider)
                .cloned()
                .ok_or_else(|| {
                    Paper2CodesError::LLM(crate::error::LLMError::ModelUnavailable(
                        "No default provider".to_string(),
                    ))
                })
        }
    }
}
