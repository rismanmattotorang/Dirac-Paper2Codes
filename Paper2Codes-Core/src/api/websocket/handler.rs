//! WebSocket connection handler
//!
//! This module handles WebSocket connections, authentication, and message processing.

use crate::api::state::AppState;
use crate::api::websocket::manager::{Connection, ConnectionId, ConnectionManager};
use crate::api::websocket::protocol::WsMessage;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Query};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Authentication token (optional for now, will be required later)
    token: Option<String>,
    /// Initial channels to subscribe to (comma-separated)
    channels: Option<String>,
}

/// Handle WebSocket upgrade request
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    Extension(state): Extension<Arc<AppState>>,
) -> Response {
    // Extract user ID from token if provided
    let user_id = if let Some(token) = params.token {
        // Verify JWT token and extract user ID
        #[cfg(feature = "api")]
        {
            match state.jwt_service.validate_token(&token) {
                Ok(claims) => Some(claims.sub),
                Err(e) => {
                    warn!(error = %e, "Invalid WebSocket token");
                    None
                }
            }
        }
        #[cfg(not(feature = "api"))]
        {
            // Without API feature, we can't verify tokens
            None
        }
    } else {
        None
    };

    // Parse initial channels
    let initial_channels: Vec<String> = params
        .channels
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();

    info!(
        user_id = ?user_id,
        channels = ?initial_channels,
        "WebSocket connection request"
    );

    // Get connection manager from state
    let connection_manager = state.websocket_manager.clone();

    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            user_id,
            initial_channels,
            connection_manager.clone(),
        )
    })
}

/// Handle WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    user_id: Option<String>,
    initial_channels: Vec<String>,
    connection_manager: Arc<ConnectionManager>,
) {
    // Create connection
    let connection = Arc::new(Connection::new(user_id.clone()));
    let connection_id = connection.id;

    // Add connection to manager
    connection_manager.add_connection(connection.clone()).await;

    // Subscribe to initial channels
    for channel in initial_channels {
        connection_manager.subscribe(connection_id, channel).await;
    }

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Clone connection for the receive task
    let connection_clone = connection.clone();
    let connection_manager_clone = connection_manager.clone();

    // Spawn task to receive messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Parse message
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(message) => {
                            handle_client_message(
                                message,
                                connection_id,
                                &connection_clone,
                                &connection_manager_clone,
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!(
                                connection_id = %connection_id,
                                error = %e,
                                "Failed to parse WebSocket message"
                            );

                            // Send error response
                            let error_msg = WsMessage::error(
                                "system",
                                "INVALID_MESSAGE",
                                format!("Failed to parse message: {}", e),
                                None,
                            );

                            if let Err(e) = connection_clone.broadcast(error_msg) {
                                error!(
                                    connection_id = %connection_id,
                                    error = %e,
                                    "Failed to send error message"
                                );
                            }
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!(
                        connection_id = %connection_id,
                        "Received binary message (not supported)"
                    );
                }
                Ok(Message::Ping(_data)) => {
                    // Respond to ping with pong
                    debug!(connection_id = %connection_id, "Received ping");
                    // Pong will be handled automatically by axum
                }
                Ok(Message::Pong(_)) => {
                    debug!(connection_id = %connection_id, "Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!(connection_id = %connection_id, "Client closed connection");
                    break;
                }
                Err(e) => {
                    error!(
                        connection_id = %connection_id,
                        error = %e,
                        "WebSocket error"
                    );
                    break;
                }
            }
        }

        // Cleanup
        connection_manager_clone
            .remove_connection(connection_id)
            .await;
    });

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        let mut receiver = connection.receiver.resubscribe();
        let mut heartbeat_interval = interval(Duration::from_secs(30)); // 30 second heartbeat

        loop {
            tokio::select! {
                // Receive message from broadcast channel
                result = receiver.recv() => {
                    match result {
                        Ok(message) => {
                            // Serialize and send message
                            match serde_json::to_string(&message) {
                                Ok(text) => {
                                    if let Err(e) = sender.send(Message::Text(text)).await {
                                        error!(
                                            connection_id = %connection_id,
                                            error = %e,
                                            "Failed to send WebSocket message"
                                        );
                                        break;
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        connection_id = %connection_id,
                                        error = %e,
                                        "Failed to serialize WebSocket message"
                                    );
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!(
                                connection_id = %connection_id,
                                skipped = skipped,
                                "WebSocket receiver lagged, messages skipped"
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            info!(
                                connection_id = %connection_id,
                                "Broadcast channel closed"
                            );
                            break;
                        }
                    }
                }
                // Send heartbeat
                _ = heartbeat_interval.tick() => {
                    let heartbeat = WsMessage::heartbeat();
                    match serde_json::to_string(&heartbeat) {
                        Ok(text) => {
                            if let Err(e) = sender.send(Message::Text(text)).await {
                                error!(
                                    connection_id = %connection_id,
                                    error = %e,
                                    "Failed to send heartbeat"
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            error!(
                                connection_id = %connection_id,
                                error = %e,
                                "Failed to serialize heartbeat"
                            );
                        }
                    }
                }
            }
        }

        // Cleanup
        connection_manager.remove_connection(connection_id).await;
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut recv_task => {
            send_task.abort();
        }
        _ = &mut send_task => {
            recv_task.abort();
        }
    }

    info!(connection_id = %connection_id, "WebSocket connection closed");
}

/// Handle client message
async fn handle_client_message(
    message: WsMessage,
    connection_id: ConnectionId,
    connection: &Connection,
    connection_manager: &ConnectionManager,
) {
    match message {
        WsMessage::Subscribe { channels } => {
            // Subscribe to channels
            let mut subscribed = Vec::new();
            for channel in channels {
                if connection_manager
                    .subscribe(connection_id, channel.clone())
                    .await
                {
                    subscribed.push(channel);
                }
            }

            // Send confirmation
            let response = WsMessage::Subscribed {
                channels: subscribed,
            };
            if let Err(e) = connection.broadcast(response) {
                warn!(
                    connection_id = %connection_id,
                    error = %e,
                    "Failed to send subscription confirmation"
                );
            }
        }
        WsMessage::Unsubscribe { channels } => {
            // Unsubscribe from channels
            let mut unsubscribed = Vec::new();
            for channel in channels {
                if connection_manager
                    .unsubscribe(connection_id, &channel)
                    .await
                {
                    unsubscribed.push(channel);
                }
            }

            // Send confirmation
            let response = WsMessage::Unsubscribed {
                channels: unsubscribed,
            };
            if let Err(e) = connection.broadcast(response) {
                warn!(
                    connection_id = %connection_id,
                    error = %e,
                    "Failed to send unsubscription confirmation"
                );
            }
        }
        _ => {
            // Other message types are server-to-client only
            warn!(
                connection_id = %connection_id,
                "Received unexpected message type from client"
            );
        }
    }
}
