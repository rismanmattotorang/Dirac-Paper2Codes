#[cfg(feature = "api")]
use crate::api::auth::middleware::auth_middleware;
#[cfg(feature = "api")]
use crate::api::cache::middleware::cache_middleware;
#[cfg(feature = "api")]
use crate::api::handlers::analytics::{
    get_agent_performance, get_analytics_overview, get_performance_metrics, get_usage_statistics,
};
#[cfg(feature = "api")]
use crate::api::handlers::auth::auth_router;
#[cfg(feature = "api")]
use crate::api::handlers::files::{
    download_file, get_file_metadata, list_file_versions, preview_file, upload_file,
};
#[cfg(feature = "api")]
use crate::api::handlers::metrics::get_metrics;
#[cfg(feature = "api")]
use crate::api::handlers::papers_phase3::{
    get_paper_segments, get_processing_status, process_paper, search_papers, upload_paper,
};
#[cfg(feature = "api")]
use crate::api::handlers::search::{advanced_search, get_search_facets, get_search_suggestions};
#[cfg(feature = "api")]
use crate::api::handlers::streaming::{
    stream_generation_progress, stream_llm_response, stream_task_logs,
};
#[cfg(feature = "api")]
use crate::api::handlers::tools::list_tools;
use crate::api::handlers::{
    health::health_check,
    papers::{create_paper, delete_paper, get_paper, list_papers, update_paper},
    repositories::{get_repository, list_repositories},
    tasks::{cancel_task, create_task, get_task, list_tasks, update_task_status},
};
use crate::api::middleware::{cors, error, logging, rate_limit, request_id, security};
use crate::api::state::AppState;
#[cfg(feature = "api")]
use crate::api::websocket::websocket_handler;
use crate::config::Config;
use crate::error::Result;
use axum::routing::delete;
use axum::routing::post;
use axum::routing::put;
use axum::{middleware, routing::get, Extension, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

/// API Server for Paper2Codes
pub struct ApiServer {
    router: Router,
}

impl ApiServer {
    /// Create a new API server instance
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing API server...");

        // Create application state
        let app_state = Arc::new(AppState::new(config.clone()).await?);

        // Create CORS layer
        let cors_layer = cors::cors_layer(&config);

        // Create rate limiter middleware
        let rate_limiter = Arc::new(rate_limit::RateLimitMiddleware::new(&config.api.rate_limit));

        // Build public routes (no authentication required)
        let mut public_routes = Router::new()
            // Health check
            .route("/api/health", get(health_check));

        #[cfg(feature = "api")]
        {
            // Metrics endpoint (public, but should be protected in production)
            public_routes = public_routes.route("/metrics", get(get_metrics));

            // Analytics endpoints (public for dashboard access)
            public_routes = public_routes
                .route("/api/analytics/overview", get(get_analytics_overview))
                .route("/api/analytics/performance", get(get_performance_metrics))
                .route("/api/analytics/usage", get(get_usage_statistics))
                .route("/api/analytics/agents", get(get_agent_performance))
                .route(
                    "/api/analytics/domains",
                    get(crate::api::handlers::analytics::get_domain_distribution),
                )
                .route(
                    "/api/analytics/llm-usage",
                    get(crate::api::handlers::analytics::get_llm_usage_breakdown),
                );
        }

        #[cfg(feature = "api")]
        {
            // Auth routes
            public_routes = public_routes.merge(auth_router());
            // WebSocket endpoint
            public_routes = public_routes.route("/ws", axum::routing::get(websocket_handler));

            // Make read-only endpoints public for dashboard access
            public_routes = public_routes
                .route("/api/papers", get(list_papers))
                .route("/api/papers/:id", get(get_paper))
                .route("/api/repositories", get(list_repositories))
                .route("/api/repositories/:id", get(get_repository))
                .route("/api/tasks", get(list_tasks))
                .route("/api/tasks/:id", get(get_task))
                .route("/api/tools", get(list_tools))
                .route(
                    "/api/settings",
                    get(crate::api::handlers::settings::get_settings),
                )
                .route(
                    "/api/settings/llm/providers",
                    get(crate::api::handlers::llm_keys::list_providers),
                )
                // Domain skills (read-only listing/detail is public)
                .route(
                    "/api/skills",
                    get(crate::api::handlers::skills::list_skills),
                )
                .route(
                    "/api/skills/:id",
                    get(crate::api::handlers::skills::get_skill),
                );
        }

        // Build protected routes (authentication required for write operations)
        let mut protected_routes = Router::new()
            // Papers write endpoints
            .route("/api/papers", post(create_paper))
            .route("/api/papers/:id", put(update_paper))
            .route("/api/papers/:id", delete(delete_paper))
            // Tasks write endpoints
            .route("/api/tasks", post(create_task))
            .route("/api/tasks/:id/status", put(update_task_status))
            .route("/api/tasks/:id/cancel", post(cancel_task));

        #[cfg(feature = "api")]
        {
            // Phase 3: Paper Processing endpoints
            protected_routes = protected_routes
                .route("/api/papers/upload", post(upload_paper))
                .route("/api/papers/:id/process", post(process_paper))
                .route("/api/papers/:id/status", get(get_processing_status))
                .route("/api/papers/:id/segments", get(get_paper_segments))
                .route("/api/papers/search", post(search_papers));

            // Phase 5: Streaming endpoints (SSE)
            protected_routes = protected_routes
                .route(
                    "/api/papers/:id/generate/stream",
                    get(stream_generation_progress),
                )
                .route("/api/tasks/:id/logs/stream", get(stream_task_logs))
                .route("/api/llm/stream", get(stream_llm_response));

            // Phase 5: File management endpoints
            protected_routes = protected_routes
                .route("/api/files/upload", post(upload_file))
                .route("/api/files/:id", get(download_file))
                .route("/api/files/:id/preview", get(preview_file))
                .route("/api/files/:id/versions", get(list_file_versions))
                .route("/api/files/:id/metadata", get(get_file_metadata));

            // Phase 5: Advanced search endpoints
            protected_routes = protected_routes
                .route("/api/search", axum::routing::post(advanced_search))
                .route("/api/search/suggestions", get(get_search_suggestions))
                .route("/api/search/facets", get(get_search_facets));

            // Module endpoints
            protected_routes = protected_routes
                .route(
                    "/api/repositories/:id/modules",
                    get(crate::api::handlers::modules::get_repository_modules),
                )
                .route(
                    "/api/repositories/:id/download",
                    get(crate::api::handlers::modules::download_repository),
                )
                .route(
                    "/api/modules/:id",
                    get(crate::api::handlers::modules::get_module_content),
                )
                .route(
                    "/api/modules/:id",
                    put(crate::api::handlers::modules::update_module_content),
                )
                .route(
                    "/api/modules/:id/download",
                    get(crate::api::handlers::modules::download_module),
                )
                .route(
                    "/api/modules/:id/verification",
                    get(crate::api::handlers::modules::get_module_verification),
                )
                .route(
                    "/api/modules/search",
                    get(crate::api::handlers::modules::search_modules),
                );

            // Phase 6: Batch request endpoint
            protected_routes = protected_routes.route(
                "/api/batch",
                axum::routing::post(crate::api::handlers::batch::process_batch),
            );

            // Settings endpoints (write operations only)
            protected_routes = protected_routes
                .route(
                    "/api/settings",
                    put(crate::api::handlers::settings::update_settings),
                )
                .route(
                    "/api/settings/llm/providers/:provider/key",
                    put(crate::api::handlers::llm_keys::set_provider_key),
                )
                .route(
                    "/api/settings/llm/providers/:provider/key",
                    delete(crate::api::handlers::llm_keys::delete_provider_key),
                )
                .route(
                    "/api/settings/llm/providers/:provider/test",
                    post(crate::api::handlers::llm_keys::test_provider_key),
                )
                .route(
                    "/api/settings/llm/default",
                    put(crate::api::handlers::llm_keys::set_default_provider),
                )
                // Domain skill upsert (create / improve a user skill)
                .route(
                    "/api/skills/:id",
                    put(crate::api::handlers::skills::upsert_skill),
                )
                .route(
                    "/api/settings/security/sessions/:id",
                    delete(crate::api::handlers::settings::revoke_session),
                )
                .route(
                    "/api/settings/security/2fa/enable",
                    post(crate::api::handlers::settings::enable_2fa),
                )
                .route(
                    "/api/settings/security/2fa/disable",
                    post(crate::api::handlers::settings::disable_2fa),
                )
                .route(
                    "/api/settings/database/test",
                    post(crate::api::handlers::settings::test_database_connection),
                )
                .route(
                    "/api/settings/team/members",
                    post(crate::api::handlers::settings::add_team_member),
                )
                .route(
                    "/api/settings/team/members/:id",
                    delete(crate::api::handlers::settings::remove_team_member),
                )
                .route(
                    "/api/settings/team/members/:id/role",
                    put(crate::api::handlers::settings::update_team_member_role),
                );

            // Apply authentication middleware to protected routes
            protected_routes = protected_routes.layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth_middleware,
            ));
        }

        // Combine all routes
        // Build service builder with all middleware layers
        // Note: Conditional compilation for cache middleware requires separate handling
        let mut router = Router::new().merge(public_routes).merge(protected_routes);

        // Apply middleware layers (order matters - bottom to top)
        // Cache middleware (Phase 6: Performance Optimization)
        #[cfg(feature = "api")]
        {
            router = router.layer(middleware::from_fn(cache_middleware));
        }

        router = router.layer(
            ServiceBuilder::new()
                // Compression (outermost - compresses response)
                .layer(CompressionLayer::new())
                // Security headers
                .layer(middleware::from_fn(security::security_headers_middleware))
                // Rate limiting
                .layer(axum::Extension(rate_limiter.clone()))
                .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
                // Error handling
                .layer(middleware::from_fn(error::error_handler))
                // Logging
                .layer(middleware::from_fn(logging::log_requests))
                // Request ID
                .layer(middleware::from_fn(request_id::add_request_id))
                // Timeout
                .layer(TimeoutLayer::new(std::time::Duration::from_secs(30)))
                // CORS
                .layer(cors_layer)
                .into_inner(),
        );
        router = router.layer(Extension(app_state.clone()));

        info!("API server initialized successfully");

        Ok(Self { router })
    }

    /// Start the API server
    pub async fn serve(self, addr: SocketAddr) -> Result<()> {
        info!("Starting API server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to bind to address {}: {}",
                addr, e
            )))
        })?;

        info!("API server listening on {}", addr);

        // Start server with graceful shutdown
        let make_service = self.router.into_make_service();

        axum::serve(listener, make_service)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| {
                crate::error::Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                    "Server error: {}",
                    e
                )))
            })?;

        info!("API server stopped");
        Ok(())
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, shutting down gracefully...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully...");
        }
    }
}
