//! LLM API key management endpoints.
//!
//! These handlers give the Web UI a first-class way to inspect and manage the
//! API keys used to talk to each LLM provider. Keys are never returned in full
//! — only a masked preview and metadata (configured / source / default) is
//! exposed. Mutations are applied to the runtime override store on
//! [`AppState`] (so they take effect immediately) and persisted to disk.

use axum::{
    extract::{Extension, Path},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

use crate::api::state::AppState;
use crate::config::{provider_env_var, provider_info, Config, KNOWN_PROVIDERS};
use crate::error::{Paper2CodesError, Result};

/// Public status of a single provider's key configuration.
#[derive(Debug, Serialize)]
pub struct ProviderStatus {
    /// Canonical provider id (map key).
    pub id: String,
    /// Human friendly display name.
    pub name: String,
    /// Whether the provider is enabled in the effective config.
    pub enabled: bool,
    /// Whether this provider is the current default.
    pub is_default: bool,
    /// API base URL in effect.
    pub base_url: Option<String>,
    /// Models offered for this provider.
    pub models: Vec<String>,
    /// Whether a usable key is currently available (config, override or env).
    pub configured: bool,
    /// Masked preview of the configured key (never the full secret).
    pub masked_key: Option<String>,
    /// Where the active key comes from: `config`, `environment` or `none`.
    pub key_source: String,
    /// Recognised key prefix, used by the UI for soft validation hints.
    pub key_prefix_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct TestKeyRequest {
    /// Optional key to test before saving. When omitted, the stored key is used.
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestKeyResponse {
    pub valid: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SetDefaultRequest {
    pub provider: String,
}

/// Mask a secret for display: keep the first 4 and last 4 characters.
fn mask_key(key: &str) -> String {
    let count = key.chars().count();
    if count <= 8 {
        return "•".repeat(count.max(1));
    }
    let prefix: String = key.chars().take(4).collect();
    let suffix: String = {
        let tail: Vec<char> = key.chars().rev().take(4).collect();
        tail.into_iter().rev().collect()
    };
    format!("{}…{}", prefix, suffix)
}

/// Validate the shape of a submitted key. Intentionally lenient — we reject
/// obviously invalid input but allow custom proxies/gateways with odd formats.
fn validate_key_format(key: &str) -> Result<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(Paper2CodesError::Validation(
            "API key cannot be empty".to_string(),
        ));
    }
    if trimmed.len() < 10 {
        return Err(Paper2CodesError::Validation(
            "API key looks too short — please paste the full key".to_string(),
        ));
    }
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Err(Paper2CodesError::Validation(
            "API key must not contain whitespace".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Build the status for a single provider from an effective config snapshot.
fn status_for(id: &str, config: &Config) -> ProviderStatus {
    let info = provider_info(id);
    let provider = config.llm.providers.get(id);

    let config_key = provider.and_then(|p| p.api_key.clone());
    let env_key = std::env::var(provider_env_var(id)).ok().filter(|k| !k.is_empty());
    let resolved = config_key.clone().or_else(|| env_key.clone());

    let key_source = if config_key.is_some() {
        "config"
    } else if env_key.is_some() {
        "environment"
    } else {
        "none"
    };

    ProviderStatus {
        id: id.to_string(),
        name: info
            .map(|i| i.display_name.to_string())
            .unwrap_or_else(|| id.to_string()),
        enabled: provider.map(|p| p.enabled).unwrap_or(false),
        is_default: config.llm.default_provider == id,
        base_url: provider
            .and_then(|p| p.base_url.clone())
            .or_else(|| info.map(|i| i.base_url.to_string())),
        models: provider
            .map(|p| p.models.clone())
            .filter(|m| !m.is_empty())
            .or_else(|| info.map(|i| i.default_models.iter().map(|m| m.to_string()).collect()))
            .unwrap_or_default(),
        configured: resolved.is_some(),
        masked_key: resolved.as_deref().map(mask_key),
        key_source: key_source.to_string(),
        key_prefix_hint: info.and_then(|i| i.key_prefixes.first().map(|p| p.to_string())),
    }
}

/// List every known provider plus any extra providers present in config.
pub async fn list_providers(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<ProviderStatus>>> {
    let config = state.effective_config().await;

    // Catalog providers first (stable order), then any extra configured ones.
    let mut ids: Vec<String> = KNOWN_PROVIDERS.iter().map(|p| p.id.to_string()).collect();
    for key in config.llm.providers.keys() {
        if !ids.iter().any(|id| id == key) {
            ids.push(key.clone());
        }
    }

    let mut statuses: Vec<ProviderStatus> = ids.iter().map(|id| status_for(id, &config)).collect();

    // Surface the most relevant providers first: default, then configured.
    statuses.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then(b.configured.cmp(&a.configured))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(Json(statuses))
}

/// Set or replace the API key for a provider.
pub async fn set_provider_key(
    Extension(state): Extension<Arc<AppState>>,
    Path(provider): Path<String>,
    Json(request): Json<SetKeyRequest>,
) -> Result<Json<ProviderStatus>> {
    let key = validate_key_format(&request.api_key)?;
    state.set_llm_api_key(&provider, key).await?;
    info!("Stored API key for provider '{}'", provider);

    let config = state.effective_config().await;
    Ok(Json(status_for(&provider, &config)))
}

/// Remove the API key for a provider.
pub async fn delete_provider_key(
    Extension(state): Extension<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<Json<ProviderStatus>> {
    // Keys provided via environment variables are deployment-level and cannot be
    // removed from the UI — only config/override keys can.
    let base_key = state
        .config
        .llm
        .providers
        .get(&provider)
        .and_then(|p| p.api_key.clone());
    let env_key = std::env::var(provider_env_var(&provider))
        .ok()
        .filter(|k| !k.is_empty());

    if base_key.is_none() && env_key.is_some() {
        return Err(Paper2CodesError::Validation(format!(
            "The {} key is provided via the {} environment variable and cannot be removed here.",
            provider,
            provider_env_var(&provider)
        )));
    }

    state.remove_llm_api_key(&provider).await?;
    info!("Removed API key for provider '{}'", provider);

    let config = state.effective_config().await;
    Ok(Json(status_for(&provider, &config)))
}

/// Set the default provider. The provider must already have a usable key.
pub async fn set_default_provider(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SetDefaultRequest>,
) -> Result<Json<Vec<ProviderStatus>>> {
    let config = state.effective_config().await;
    if config.get_api_key(&request.provider).is_none() {
        return Err(Paper2CodesError::Validation(format!(
            "Cannot set '{}' as default: no API key is configured for it.",
            request.provider
        )));
    }

    state.set_default_llm_provider(request.provider.clone()).await?;
    info!("Default LLM provider set to '{}'", request.provider);

    list_providers(Extension(state)).await
}

/// Validate a key by making a lightweight, read-only request to the provider.
pub async fn test_provider_key(
    Extension(state): Extension<Arc<AppState>>,
    Path(provider): Path<String>,
    Json(request): Json<TestKeyRequest>,
) -> Result<Json<TestKeyResponse>> {
    let config = state.effective_config().await;

    let key = match request.api_key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
        Some(key) => key,
        None => config.get_api_key(&provider).ok_or_else(|| {
            Paper2CodesError::Validation(format!(
                "No API key configured for provider '{}'",
                provider
            ))
        })?,
    };

    let base_url = config
        .llm
        .providers
        .get(&provider)
        .and_then(|p| p.base_url.clone())
        .or_else(|| provider_info(&provider).map(|i| i.base_url.to_string()))
        .ok_or_else(|| {
            Paper2CodesError::Validation(format!("Unknown provider '{}'", provider))
        })?;

    Ok(Json(test_key_live(&provider, &base_url, &key).await))
}

/// Issue a minimal authenticated request to confirm a key works.
///
/// All supported providers expose a `GET /models` listing endpoint, which is a
/// cheap, side-effect-free way to verify credentials.
async fn test_key_live(provider: &str, base_url: &str, key: &str) -> TestKeyResponse {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return TestKeyResponse {
                valid: false,
                message: format!("Failed to build HTTP client: {}", e),
                latency_ms: None,
            }
        }
    };

    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let builder = if provider == "anthropic" {
        client
            .get(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    } else {
        client.get(&url).header("Authorization", format!("Bearer {}", key))
    };

    let start = Instant::now();
    match builder.send().await {
        Ok(response) => {
            let latency = start.elapsed().as_millis() as u64;
            let status = response.status();
            if status.is_success() {
                TestKeyResponse {
                    valid: true,
                    message: format!("Key is valid (HTTP {}).", status.as_u16()),
                    latency_ms: Some(latency),
                }
            } else if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                TestKeyResponse {
                    valid: false,
                    message: "Authentication failed — the API key was rejected.".to_string(),
                    latency_ms: Some(latency),
                }
            } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                TestKeyResponse {
                    valid: true,
                    message: "Key accepted but currently rate-limited (HTTP 429).".to_string(),
                    latency_ms: Some(latency),
                }
            } else {
                TestKeyResponse {
                    valid: false,
                    message: format!("Unexpected response from provider: HTTP {}.", status.as_u16()),
                    latency_ms: Some(latency),
                }
            }
        }
        Err(e) => TestKeyResponse {
            valid: false,
            message: format!("Could not reach provider: {}", e),
            latency_ms: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_long_keys() {
        assert_eq!(mask_key("sk-abcdefghijklmnop"), "sk-a…mnop");
    }

    #[test]
    fn masks_short_keys_fully() {
        assert_eq!(mask_key("short"), "•••••");
        assert_eq!(mask_key(""), "•");
    }

    #[test]
    fn rejects_empty_and_short_keys() {
        assert!(validate_key_format("   ").is_err());
        assert!(validate_key_format("sk-123").is_err());
        assert!(validate_key_format("sk-abc def ghi").is_err());
        assert!(validate_key_format("sk-proj-abcdefghijklmnop").is_ok());
    }
}
