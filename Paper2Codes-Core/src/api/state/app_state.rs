#[cfg(feature = "api")]
use crate::api::auth::JwtService;
#[cfg(feature = "api")]
use crate::api::cache::InMemoryCache;
#[cfg(feature = "api")]
use crate::api::websocket::manager::ConnectionManager;
use crate::config::Config;
use crate::storage::errors::StorageError;
use crate::storage::StorageManager;
use std::sync::Arc;
#[cfg(feature = "api")]
use std::time::Duration;
use tokio::sync::RwLock;

/// Shared application state accessible to all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    pub config: Arc<Config>,
    /// Storage manager for database operations
    pub storage: Arc<RwLock<Option<StorageManager>>>,
    /// JWT service for authentication
    #[cfg(feature = "api")]
    pub jwt_service: Arc<JwtService>,
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

        #[cfg(feature = "api")]
        let websocket_manager = Arc::new(ConnectionManager::new());

        #[cfg(feature = "api")]
        let cache = Arc::new(InMemoryCache::new(
            config.api.cache.max_entries,
            Duration::from_secs(config.api.cache.default_ttl),
        ));

        Ok(Self {
            config: Arc::new(config),
            storage: Arc::new(RwLock::new(storage)),
            #[cfg(feature = "api")]
            jwt_service,
            #[cfg(feature = "api")]
            websocket_manager,
            #[cfg(feature = "api")]
            cache,
        })
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
