//! Prometheus metrics for API monitoring
//!
//! This module provides Prometheus metrics collection for monitoring
//! API performance, request rates, error rates, and system health.

#[cfg(feature = "api")]
use lazy_static::lazy_static;
#[cfg(feature = "api")]
use prometheus::{Counter, Encoder, Gauge, Histogram, HistogramOpts, Opts, Registry, TextEncoder};

#[cfg(feature = "api")]
lazy_static! {
    /// Total number of HTTP requests
    pub static ref HTTP_REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("http_requests_total", "Total number of HTTP requests")
            .namespace("paper2codes")
            .subsystem("api")
    ).expect("metric can be created");

    /// HTTP request duration in seconds
    pub static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        )
        .namespace("paper2codes")
        .subsystem("api")
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    ).expect("metric can be created");

    /// Total number of HTTP request errors
    pub static ref HTTP_REQUEST_ERRORS: Counter = Counter::with_opts(
        Opts::new("http_request_errors_total", "Total number of HTTP request errors")
            .namespace("paper2codes")
            .subsystem("api")
    ).expect("metric can be created");

    /// Number of active HTTP connections
    pub static ref ACTIVE_CONNECTIONS: Gauge = Gauge::with_opts(
        Opts::new("active_connections", "Number of active HTTP connections")
            .namespace("paper2codes")
            .subsystem("api")
    ).expect("metric can be created");

    /// Number of active WebSocket connections
    pub static ref WEBSOCKET_CONNECTIONS: Gauge = Gauge::with_opts(
        Opts::new("websocket_connections", "Number of active WebSocket connections")
            .namespace("paper2codes")
            .subsystem("api")
    ).expect("metric can be created");

    /// Database query duration in seconds
    pub static ref DATABASE_QUERY_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "database_query_duration_seconds",
            "Database query duration in seconds"
        )
        .namespace("paper2codes")
        .subsystem("storage")
        .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0])
    ).expect("metric can be created");

    /// Total number of cache hits
    pub static ref CACHE_HITS_TOTAL: Counter = Counter::with_opts(
        Opts::new("cache_hits_total", "Total number of cache hits")
            .namespace("paper2codes")
            .subsystem("cache")
    ).expect("metric can be created");

    /// Total number of cache misses
    pub static ref CACHE_MISSES_TOTAL: Counter = Counter::with_opts(
        Opts::new("cache_misses_total", "Total number of cache misses")
            .namespace("paper2codes")
            .subsystem("cache")
    ).expect("metric can be created");

    /// Number of active tasks
    pub static ref ACTIVE_TASKS: Gauge = Gauge::with_opts(
        Opts::new("active_tasks", "Number of active tasks")
            .namespace("paper2codes")
            .subsystem("coordinator")
    ).expect("metric can be created");

    /// Total number of tasks completed
    pub static ref TASKS_COMPLETED_TOTAL: Counter = Counter::with_opts(
        Opts::new("tasks_completed_total", "Total number of tasks completed")
            .namespace("paper2codes")
            .subsystem("coordinator")
    ).expect("metric can be created");

    /// Total number of tasks failed
    pub static ref TASKS_FAILED_TOTAL: Counter = Counter::with_opts(
        Opts::new("tasks_failed_total", "Total number of tasks failed")
            .namespace("paper2codes")
            .subsystem("coordinator")
    ).expect("metric can be created");

    /// LLM API request duration in seconds
    pub static ref LLM_REQUEST_DURATION: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "llm_request_duration_seconds",
            "LLM API request duration in seconds"
        )
        .namespace("paper2codes")
        .subsystem("llm")
        .buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0])
    ).expect("metric can be created");

    /// Total number of LLM API requests
    pub static ref LLM_REQUESTS_TOTAL: Counter = Counter::with_opts(
        Opts::new("llm_requests_total", "Total number of LLM API requests")
            .namespace("paper2codes")
            .subsystem("llm")
    ).expect("metric can be created");

    /// Total number of LLM API errors
    pub static ref LLM_ERRORS_TOTAL: Counter = Counter::with_opts(
        Opts::new("llm_errors_total", "Total number of LLM API errors")
            .namespace("paper2codes")
            .subsystem("llm")
    ).expect("metric can be created");

    /// Prometheus metrics registry
    pub static ref REGISTRY: Registry = {
        let registry = Registry::new();

        // Register all metrics
        registry.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
        registry.register(Box::new(HTTP_REQUEST_ERRORS.clone())).unwrap();
        registry.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
        registry.register(Box::new(WEBSOCKET_CONNECTIONS.clone())).unwrap();
        registry.register(Box::new(DATABASE_QUERY_DURATION.clone())).unwrap();
        registry.register(Box::new(CACHE_HITS_TOTAL.clone())).unwrap();
        registry.register(Box::new(CACHE_MISSES_TOTAL.clone())).unwrap();
        registry.register(Box::new(ACTIVE_TASKS.clone())).unwrap();
        registry.register(Box::new(TASKS_COMPLETED_TOTAL.clone())).unwrap();
        registry.register(Box::new(TASKS_FAILED_TOTAL.clone())).unwrap();
        registry.register(Box::new(LLM_REQUEST_DURATION.clone())).unwrap();
        registry.register(Box::new(LLM_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(LLM_ERRORS_TOTAL.clone())).unwrap();

        registry
    };
}

/// Get metrics in Prometheus text format
#[cfg(feature = "api")]
pub fn gather_metrics() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// Record HTTP request metrics
#[cfg(feature = "api")]
pub fn record_http_request(duration_seconds: f64, is_error: bool) {
    HTTP_REQUESTS_TOTAL.inc();
    HTTP_REQUEST_DURATION.observe(duration_seconds);
    if is_error {
        HTTP_REQUEST_ERRORS.inc();
    }
}

/// Record cache hit
#[cfg(feature = "api")]
pub fn record_cache_hit() {
    CACHE_HITS_TOTAL.inc();
}

/// Record cache miss
#[cfg(feature = "api")]
pub fn record_cache_miss() {
    CACHE_MISSES_TOTAL.inc();
}

/// Record database query duration
#[cfg(feature = "api")]
pub fn record_database_query(duration_seconds: f64) {
    DATABASE_QUERY_DURATION.observe(duration_seconds);
}

/// Record LLM request metrics
#[cfg(feature = "api")]
pub fn record_llm_request(duration_seconds: f64, is_error: bool) {
    LLM_REQUESTS_TOTAL.inc();
    LLM_REQUEST_DURATION.observe(duration_seconds);
    if is_error {
        LLM_ERRORS_TOTAL.inc();
    }
}

/// Update active connections count
#[cfg(feature = "api")]
pub fn set_active_connections(count: f64) {
    ACTIVE_CONNECTIONS.set(count);
}

/// Update WebSocket connections count
#[cfg(feature = "api")]
pub fn set_websocket_connections(count: f64) {
    WEBSOCKET_CONNECTIONS.set(count);
}

/// Update active tasks count
#[cfg(feature = "api")]
pub fn set_active_tasks(count: f64) {
    ACTIVE_TASKS.set(count);
}

/// Record task completion
#[cfg(feature = "api")]
pub fn record_task_completed() {
    TASKS_COMPLETED_TOTAL.inc();
}

/// Record task failure
#[cfg(feature = "api")]
pub fn record_task_failed() {
    TASKS_FAILED_TOTAL.inc();
}
