//! WebSocket connection manager
//!
//! This module manages WebSocket connections, subscriptions, and message broadcasting.

use crate::api::websocket::protocol::WsMessage;
// Note: SinkExt and StreamExt are not needed in manager.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Connection ID type
pub type ConnectionId = Uuid;

/// Active WebSocket connection
pub struct Connection {
    pub id: ConnectionId,
    pub user_id: Option<String>,
    pub channels: Arc<RwLock<HashSet<String>>>,
    pub sender: broadcast::Sender<WsMessage>,
    pub receiver: broadcast::Receiver<WsMessage>,
}

impl Connection {
    /// Create a new connection
    pub fn new(user_id: Option<String>) -> Self {
        let id = Uuid::new_v4();
        let (sender, receiver) = broadcast::channel(1000); // Buffer up to 1000 messages

        Self {
            id,
            user_id,
            channels: Arc::new(RwLock::new(HashSet::new())),
            sender,
            receiver,
        }
    }

    /// Subscribe to a channel
    pub async fn subscribe(&self, channel: String) -> bool {
        let mut channels = self.channels.write().await;
        channels.insert(channel)
    }

    /// Unsubscribe from a channel
    pub async fn unsubscribe(&self, channel: &str) -> bool {
        let mut channels = self.channels.write().await;
        channels.remove(channel)
    }

    /// Check if subscribed to a channel
    pub async fn is_subscribed(&self, channel: &str) -> bool {
        let channels = self.channels.read().await;
        channels.contains(channel)
    }

    /// Get all subscribed channels
    pub async fn get_channels(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.iter().cloned().collect()
    }

    /// Broadcast a message to this connection
    pub fn broadcast(
        &self,
        message: WsMessage,
    ) -> Result<usize, broadcast::error::SendError<WsMessage>> {
        self.sender.send(message)
    }
}

/// WebSocket connection manager
#[derive(Clone)]
pub struct ConnectionManager {
    /// Active connections by connection ID
    connections: Arc<RwLock<HashMap<ConnectionId, Arc<Connection>>>>,
    /// Connections by user ID
    user_connections: Arc<RwLock<HashMap<String, HashSet<ConnectionId>>>>,
    /// Connections by channel
    channel_connections: Arc<RwLock<HashMap<String, HashSet<ConnectionId>>>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            channel_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new connection
    pub async fn add_connection(&self, connection: Arc<Connection>) {
        let connection_id = connection.id;
        let user_id = connection.user_id.clone();

        // Add to connections map
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, connection.clone());
        }

        // Add to user connections if user_id is present
        if let Some(ref user_id) = user_id {
            let mut user_conns = self.user_connections.write().await;
            user_conns
                .entry(user_id.clone())
                .or_insert_with(HashSet::new)
                .insert(connection_id);
        }

        info!(
            connection_id = %connection_id,
            user_id = ?user_id,
            "WebSocket connection added"
        );
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: ConnectionId) {
        // Get connection to access user_id and channels
        let (user_id, channels) = {
            let connections = self.connections.read().await;
            if let Some(conn) = connections.get(&connection_id) {
                (conn.user_id.clone(), conn.get_channels().await)
            } else {
                return;
            }
        };

        // Remove from connections map
        {
            let mut connections = self.connections.write().await;
            connections.remove(&connection_id);
        }

        // Remove from user connections
        if let Some(user_id) = user_id {
            let mut user_conns = self.user_connections.write().await;
            if let Some(conns) = user_conns.get_mut(&user_id) {
                conns.remove(&connection_id);
                if conns.is_empty() {
                    user_conns.remove(&user_id);
                }
            }
        }

        // Remove from channel connections
        {
            let mut channel_conns = self.channel_connections.write().await;
            for channel in channels {
                if let Some(conns) = channel_conns.get_mut(&channel) {
                    conns.remove(&connection_id);
                    if conns.is_empty() {
                        channel_conns.remove(&channel);
                    }
                }
            }
        }

        info!(
            connection_id = %connection_id,
            "WebSocket connection removed"
        );
    }

    /// Subscribe a connection to a channel
    pub async fn subscribe(&self, connection_id: ConnectionId, channel: String) -> bool {
        // Get connection
        let connection = {
            let connections = self.connections.read().await;
            connections.get(&connection_id).cloned()
        };

        if let Some(conn) = connection {
            // Subscribe in connection
            let subscribed = conn.subscribe(channel.clone()).await;

            if subscribed {
                // Add to channel connections
                let channel_for_log = channel.clone();
                let mut channel_conns = self.channel_connections.write().await;
                channel_conns
                    .entry(channel)
                    .or_insert_with(HashSet::new)
                    .insert(connection_id);

                debug!(
                    connection_id = %connection_id,
                    channel = %channel_for_log,
                    "Connection subscribed to channel"
                );
            }

            subscribed
        } else {
            false
        }
    }

    /// Unsubscribe a connection from a channel
    pub async fn unsubscribe(&self, connection_id: ConnectionId, channel: &str) -> bool {
        // Get connection
        let connection = {
            let connections = self.connections.read().await;
            connections.get(&connection_id).cloned()
        };

        if let Some(conn) = connection {
            // Unsubscribe in connection
            let unsubscribed = conn.unsubscribe(channel).await;

            if unsubscribed {
                // Remove from channel connections
                let mut channel_conns = self.channel_connections.write().await;
                if let Some(conns) = channel_conns.get_mut(channel) {
                    conns.remove(&connection_id);
                    if conns.is_empty() {
                        channel_conns.remove(channel);
                    }
                }

                debug!(
                    connection_id = %connection_id,
                    channel = %channel,
                    "Connection unsubscribed from channel"
                );
            }

            unsubscribed
        } else {
            false
        }
    }

    /// Broadcast a message to all connections subscribed to the message's channel
    pub async fn broadcast(&self, message: WsMessage) {
        let channel = message.channel();
        let mut sent_count = 0;
        let mut error_count = 0;

        if let Some(channel) = channel {
            // Get connections for this channel
            let connection_ids = {
                let channel_conns = self.channel_connections.read().await;
                channel_conns
                    .get(channel)
                    .map(|conns| conns.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            };

            // Broadcast to each connection
            let connections = {
                let connections = self.connections.read().await;
                connection_ids
                    .iter()
                    .filter_map(|id| connections.get(id).cloned())
                    .collect::<Vec<_>>()
            };

            for conn in connections {
                match conn.broadcast(message.clone()) {
                    Ok(_) => sent_count += 1,
                    Err(e) => {
                        error_count += 1;
                        warn!(
                            connection_id = %conn.id,
                            error = %e,
                            "Failed to broadcast message to connection"
                        );
                    }
                }
            }
        }

        debug!(
            channel = ?channel,
            sent = sent_count,
            errors = error_count,
            "Broadcast message"
        );
    }

    /// Broadcast a message to a specific user's connections
    pub async fn broadcast_to_user(&self, user_id: &str, message: WsMessage) {
        // Get user's connections
        let connection_ids = {
            let user_conns = self.user_connections.read().await;
            user_conns
                .get(user_id)
                .map(|conns| conns.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        // Broadcast to each connection
        let connections = {
            let connections = self.connections.read().await;
            connection_ids
                .iter()
                .filter_map(|id| connections.get(id).cloned())
                .collect::<Vec<_>>()
        };

        for conn in connections {
            // Only send if subscribed to the channel
            if let Some(channel) = message.channel() {
                if conn.is_subscribed(channel).await {
                    if let Err(e) = conn.broadcast(message.clone()) {
                        warn!(
                            connection_id = %conn.id,
                            user_id = %user_id,
                            error = %e,
                            "Failed to broadcast message to user connection"
                        );
                    }
                }
            }
        }
    }

    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Get channel subscription count
    pub async fn channel_subscription_count(&self, channel: &str) -> usize {
        let channel_conns = self.channel_connections.read().await;
        channel_conns
            .get(channel)
            .map(|conns| conns.len())
            .unwrap_or(0)
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
