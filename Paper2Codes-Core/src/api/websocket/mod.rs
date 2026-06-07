//! WebSocket module for real-time communication
//!
//! This module provides WebSocket server functionality for real-time updates
//! including task progress, status changes, and live logs.
//!
//! ## Features
//!
//! - Connection management with authentication
//! - Channel-based subscriptions
//! - Message broadcasting
//! - Heartbeat keepalive
//! - Real-time task and paper processing updates

#[cfg(feature = "api")]
pub mod broadcast;
#[cfg(feature = "api")]
pub mod handler;
#[cfg(feature = "api")]
pub mod manager;
#[cfg(feature = "api")]
pub mod protocol;

#[cfg(feature = "api")]
pub use broadcast::*;
#[cfg(feature = "api")]
pub use handler::websocket_handler;
#[cfg(feature = "api")]
pub use manager::ConnectionManager;
#[cfg(feature = "api")]
pub use protocol::{channels, WsMessage};
