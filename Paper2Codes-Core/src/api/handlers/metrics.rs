//! Metrics endpoint handler for Prometheus
//!
//! Provides `/metrics` endpoint for Prometheus scraping

#[cfg(feature = "api")]
use crate::api::metrics;
#[cfg(feature = "api")]
use axum::http::{header, StatusCode};
#[cfg(feature = "api")]
use axum::response::Response;

/// Prometheus metrics endpoint
///
/// Returns metrics in Prometheus text format
/// GET /metrics
#[cfg(feature = "api")]
pub async fn get_metrics() -> Result<Response<String>, StatusCode> {
    match metrics::gather_metrics() {
        Ok(metrics_text) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
                .body(metrics_text)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
