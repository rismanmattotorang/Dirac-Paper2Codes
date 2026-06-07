//! API module for Paper2Codes HTTP server
//!
//! This module provides RESTful API and WebSocket endpoints for integrating
//! with the WebUI frontend. It includes authentication, middleware, handlers,
//! and real-time communication capabilities.

#[cfg(feature = "api")]
pub mod auth;
#[cfg(feature = "api")]
pub mod cache;
pub mod handlers;
#[cfg(feature = "api")]
pub mod metrics;
pub mod middleware;
pub mod server;
pub mod state;
pub mod types;
pub mod utils;
pub mod websocket;

pub use server::ApiServer;
pub use state::AppState;
