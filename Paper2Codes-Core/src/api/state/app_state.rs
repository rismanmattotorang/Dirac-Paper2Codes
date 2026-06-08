#[cfg(feature = "api")]
use crate::api::auth::JwtService;
#[cfg(feature = "api")]
use crate::api::cache::InMemoryCache;
#[cfg(feature = "api")]
use crate::api::websocket::manager::ConnectionManager;
use crate::config::Config;
use crate::storage::errors::StorageError;
use crate::storage::StorageManager;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "api")]
use std::time::Duration;
use tokio::sync::RwLock;

/// Runtime LLM overrides layered on top of the immutable bootstrap [`Config`].
///
/// The bootstrap config is loaded once at startup and shared as `Arc<Config>`,
/// so it cannot be mutated in place without an expensive lock on every reader.
/// Instead, mutable LLM settings (API keys and the default provider) are kept
/// here behind a single lock and merged on demand via
/// [`AppState::effective_config`]. This lets the Web UI change keys and have
/// them take effect immediately for in-process consumers (e.g. the SSE LLM
/// endpoint) while also being persisted to disk for worker restarts.
#[derive(Debug, Default, Clone)]
pub struct LlmOverrides {
    /// provider id -> overridden key. `Some` sets/replaces a key, `None`
    /// explicitly clears a key that was present in the bootstrap config.
    pub api_keys: HashMap<String, Option<String>>,
    /// Overridden default provider id.
    pub default_provider: Option<String>,
}

/// Absolute path of the persisted config file (`<config-dir>/paper2codes/config.toml`).
pub fn config_file_path() -> crate::error::Result<PathBuf> {
    let dir = dirs::config_dir().ok_or_else(|| {
        crate::error::Paper2CodesError::Config(crate::error::ConfigError::NotFound(
            "config directory".to_string(),
        ))
    })?;
    Ok(dir.join("paper2codes").join("config.toml"))
}

/// Shared application state accessible to all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Immutable bootstrap configuration loaded at startup.
    pub config: Arc<Config>,
    /// Mutable LLM overrides (API keys / default provider) applied at runtime.
    pub llm_overrides: Arc<RwLock<LlmOverrides>>,
    /// Storage manager for database operations
    pub storage: Arc<RwLock<Option<StorageManager>>>,
    /// JWT service for authentication
    #[cfg(feature = "api")]
    pub jwt_service: Arc<JwtService>,
    /// Stateful user store (auth)
    #[cfg(feature = "api")]
    pub user_store: Arc<crate::api::auth::UserStore>,
    /// Stateful refresh-token session store (auth)
    #[cfg(feature = "api")]
    pub session_store: Arc<crate::api::auth::SessionStore>,
    /// Personal API token store (programmatic auth)
    #[cfg(feature = "api")]
    pub api_token_store: Arc<crate::api::auth::ApiTokenStore>,
    /// Durable job queue for long-running work (e.g. paper generation)
    pub job_queue: Arc<crate::jobs::JobQueue>,
    /// WebSocket connection manager
    #[cfg(feature = "api")]
    pub websocket_manager: Arc<ConnectionManager>,
    /// Response cache for API endpoints
    #[cfg(feature = "api")]
    pub cache: Arc<InMemoryCache>,
}

impl AppState {
    /// Create a new application state
    pub async fn new(config: Config) -> crate::error::Result<Self> {
        let storage = if config.storage.enabled {
            tracing::info!(
                "Initializing storage connection to {}",
                config.storage.connection_string
            );
            let manager = StorageManager::new();
            let storage_config = config.storage.clone();

            match manager.connect(storage_config.clone()).await {
                Ok(_) => {
                    tracing::info!(
                        "Successfully connected to SurrealDB at {} (namespace: {}, database: {})",
                        storage_config.connection_string,
                        storage_config.namespace,
                        storage_config.database
                    );

                    // Test the connection
                    if let Err(e) = manager.test_connection().await {
                        tracing::error!("Storage connection test failed: {}", e);
                        return Err(crate::error::Paper2CodesError::Storage(
                            crate::storage::errors::StorageError::Connection(format!(
                                "Connection test failed: {}",
                                e
                            )),
                        ));
                    }
                    tracing::info!("Storage connection test passed");

                    Some(manager)
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to connect to SurrealDB at {}: {}. Storage will be disabled.",
                        storage_config.connection_string,
                        e
                    );
                    tracing::warn!(
                        "Make sure SurrealDB is running and accessible at {}",
                        storage_config.connection_string
                    );
                    None
                }
            }
        } else {
            tracing::info!("Storage is disabled in configuration");
            None
        };

        #[cfg(feature = "api")]
        let jwt_service = Arc::new(JwtService::new(&config.api.auth)?);

        // A connected storage manager (clone shares the same connection) used to
        // back the auth stores and job queue with SurrealDB for cross-restart
        // durability. `None` keeps everything in-memory (tests / no DB).
        #[cfg(feature = "api")]
        let auth_persistence: Option<Arc<dyn crate::api::auth::AuthPersistence>> = storage
            .as_ref()
            .map(|m| Arc::new(m.clone()) as Arc<dyn crate::api::auth::AuthPersistence>);
        #[cfg(feature = "api")]
        let token_persistence: Option<Arc<dyn crate::api::auth::ApiTokenPersistence>> = storage
            .as_ref()
            .map(|m| Arc::new(m.clone()) as Arc<dyn crate::api::auth::ApiTokenPersistence>);
        let job_persistence: Option<Arc<dyn crate::jobs::JobStore>> = storage
            .as_ref()
            .map(|m| Arc::new(m.clone()) as Arc<dyn crate::jobs::JobStore>);

        #[cfg(feature = "api")]
        let user_store = {
            let mut store = crate::api::auth::UserStore::new();
            if let Some(p) = auth_persistence.clone() {
                store = store.with_persistence(p);
            }
            let store = Arc::new(store);
            // Rehydrate existing accounts from the durable store.
            match store.restore().await {
                Ok(n) if n > 0 => tracing::info!("Restored {} user(s) from storage", n),
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed to restore users from storage: {}", e),
            }
            store
        };
        #[cfg(feature = "api")]
        {
            // Seed an initial admin from env (ADMIN_USERNAME / ADMIN_PASSWORD,
            // optional ADMIN_EMAIL) so a fresh deployment has a privileged account
            // without baking credentials into the image. Skipped if an account
            // with that username already exists (e.g. restored from storage), so
            // restarts don't create duplicates.
            if let (Ok(username), Ok(password)) =
                (std::env::var("ADMIN_USERNAME"), std::env::var("ADMIN_PASSWORD"))
            {
                if user_store.find_by_username(&username).await.is_some() {
                    tracing::info!("Admin user '{}' already exists; skipping seed", username);
                } else {
                    let email = std::env::var("ADMIN_EMAIL")
                        .unwrap_or_else(|_| format!("{}@local", username));
                    match crate::api::auth::User::new(
                        email,
                        username.clone(),
                        &password,
                        vec![crate::api::auth::UserRole::Admin],
                    ) {
                        Ok(user) => {
                            user_store.upsert(user).await;
                            tracing::info!("Seeded admin user '{}'", username);
                        }
                        Err(e) => tracing::warn!("Failed to seed admin user: {}", e),
                    }
                }
            }
        }
        #[cfg(feature = "api")]
        let session_store = {
            let mut store = crate::api::auth::SessionStore::new();
            if let Some(p) = auth_persistence.clone() {
                store = store.with_persistence(p);
            }
            let store = Arc::new(store);
            match store.restore().await {
                Ok(n) if n > 0 => tracing::info!("Restored {} active session(s) from storage", n),
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed to restore sessions from storage: {}", e),
            }
            store
        };
        #[cfg(feature = "api")]
        let api_token_store = {
            let mut store = crate::api::auth::ApiTokenStore::new();
            if let Some(p) = token_persistence {
                store = store.with_persistence(p);
            }
            let store = Arc::new(store);
            match store.restore().await {
                Ok(n) if n > 0 => tracing::info!("Restored {} API token(s) from storage", n),
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed to restore API tokens from storage: {}", e),
            }
            store
        };

        let job_queue = {
            let mut queue = crate::jobs::JobQueue::new();
            if let Some(p) = job_persistence {
                queue = queue.with_persistence(p);
            }
            let queue = Arc::new(queue);
            // Rehydrate outstanding jobs; Running jobs orphaned by a crash are
            // reset to Pending so the worker re-claims them.
            match queue.restore().await {
                Ok(n) if n > 0 => tracing::info!("Recovered {} outstanding job(s) from storage", n),
                Ok(_) => {}
                Err(e) => tracing::warn!("Failed to recover jobs from storage: {}", e),
            }
            queue
        };

        #[cfg(feature = "api")]
        let websocket_manager = Arc::new(ConnectionManager::new());

        #[cfg(feature = "api")]
        let cache = Arc::new(InMemoryCache::new(
            config.api.cache.max_entries,
            Duration::from_secs(config.api.cache.default_ttl),
        ));

        Ok(Self {
            config: Arc::new(config),
            llm_overrides: Arc::new(RwLock::new(LlmOverrides::default())),
            storage: Arc::new(RwLock::new(storage)),
            #[cfg(feature = "api")]
            jwt_service,
            #[cfg(feature = "api")]
            user_store,
            #[cfg(feature = "api")]
            session_store,
            #[cfg(feature = "api")]
            api_token_store,
            job_queue,
            #[cfg(feature = "api")]
            websocket_manager,
            #[cfg(feature = "api")]
            cache,
        })
    }

    /// Produce the effective configuration: the bootstrap config with the
    /// current runtime LLM overrides merged in.
    ///
    /// This is the config that should be used whenever an `LLMRouter` is built
    /// at request time so that key changes made through the Web UI take effect
    /// without a process restart.
    pub async fn effective_config(&self) -> Config {
        let mut config = (*self.config).clone();
        let overrides = self.llm_overrides.read().await;

        if let Some(provider) = &overrides.default_provider {
            config.llm.default_provider = provider.clone();
        }

        for (name, key) in &overrides.api_keys {
            match key {
                Some(value) => {
                    let entry = config
                        .llm
                        .providers
                        .entry(name.clone())
                        .or_insert_with(|| crate::config::ProviderConfig::for_provider(name));
                    entry.api_key = Some(value.clone());
                    entry.enabled = true;
                }
                None => {
                    if let Some(entry) = config.llm.providers.get_mut(name) {
                        entry.api_key = None;
                        entry.enabled = false;
                    }
                }
            }
        }

        config
    }

    /// Set or replace the API key for a provider and persist the change.
    pub async fn set_llm_api_key(&self, provider: &str, key: String) -> crate::error::Result<()> {
        self.llm_overrides
            .write()
            .await
            .api_keys
            .insert(provider.to_string(), Some(key));
        self.persist_effective_config().await.map(|_| ())
    }

    /// Clear the API key for a provider and persist the change.
    pub async fn remove_llm_api_key(&self, provider: &str) -> crate::error::Result<()> {
        self.llm_overrides
            .write()
            .await
            .api_keys
            .insert(provider.to_string(), None);
        self.persist_effective_config().await.map(|_| ())
    }

    /// Set the default LLM provider and persist the change.
    pub async fn set_default_llm_provider(&self, provider: String) -> crate::error::Result<()> {
        self.llm_overrides.write().await.default_provider = Some(provider);
        self.persist_effective_config().await.map(|_| ())
    }

    /// Serialise the effective configuration to the on-disk config file.
    ///
    /// On Unix the file is written with `0600` permissions because it can
    /// contain plaintext API keys.
    pub async fn persist_effective_config(&self) -> crate::error::Result<PathBuf> {
        let config = self.effective_config().await;
        let path = config_file_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                    "Failed to create config directory: {}",
                    e
                )))
            })?;
        }

        let toml_content = toml::to_string_pretty(&config).map_err(|e| {
            crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to serialize config: {}",
                e
            )))
        })?;

        std::fs::write(&path, toml_content).map_err(|e| {
            crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to write config file: {}",
                e
            )))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            {
                tracing::warn!("Failed to tighten permissions on {}: {}", path.display(), e);
            }
        }

        tracing::info!("Configuration persisted to {}", path.display());
        Ok(path)
    }

    /// Get reference to storage manager for operations
    pub async fn with_storage<F, Fut, R>(&self, f: F) -> crate::error::Result<R>
    where
        F: FnOnce(StorageManager) -> Fut,
        Fut: std::future::Future<Output = crate::error::Result<R>>,
    {
        let storage = {
            let storage_guard = self.storage.read().await;
            storage_guard.as_ref().cloned()
        }
        .ok_or_else(|| crate::error::Paper2CodesError::Storage(StorageError::NotConnected))?;

        f(storage).await
    }
}
