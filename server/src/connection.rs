use chrono::Utc;
use nat_traversal_common::{
    error::{NatError, NatResult},
    protocol::{ErrorCode, Message, TunnelInfo, TunnelProtocol},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_rustls::TlsStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type SecureStream = TlsStream<TcpStream>;

/// Represents a client connection to the server
pub struct ClientConnection {
    pub id: String,
    pub addr: SocketAddr,
    pub authenticated: bool,
    pub tunnels: Arc<RwLock<HashMap<Uuid, TunnelInfo>>>,
    pub sender: mpsc::UnboundedSender<Message>,
    pub bytes_sent: Arc<RwLock<u64>>,
    pub bytes_received: Arc<RwLock<u64>>,
    pub connected_at: chrono::DateTime<Utc>,
}

impl ClientConnection {
    pub fn new(id: String, addr: SocketAddr, sender: mpsc::UnboundedSender<Message>) -> Self {
        Self {
            id,
            addr,
            authenticated: false,
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            sender,
            bytes_sent: Arc::new(RwLock::new(0)),
            bytes_received: Arc::new(RwLock::new(0)),
            connected_at: Utc::now(),
        }
    }

    pub async fn send_message(&self, message: Message) -> NatResult<()> {
        self.sender
            .send(message)
            .map_err(|_| NatError::connection("Failed to send message to client"))?;
        Ok(())
    }

    pub async fn add_tunnel(&self, tunnel: TunnelInfo) {
        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(tunnel.id, tunnel);
    }

    pub async fn remove_tunnel(&self, tunnel_id: &Uuid) -> Option<TunnelInfo> {
        let mut tunnels = self.tunnels.write().await;
        tunnels.remove(tunnel_id)
    }

    pub async fn get_tunnel(&self, tunnel_id: &Uuid) -> Option<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(tunnel_id).cloned()
    }

    pub async fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }

    pub async fn update_bytes_sent(&self, bytes: u64) {
        let mut sent = self.bytes_sent.write().await;
        *sent += bytes;
    }

    pub async fn update_bytes_received(&self, bytes: u64) {
        let mut received = self.bytes_received.write().await;
        *received += bytes;
    }

    pub async fn get_stats(&self) -> (u64, u64) {
        let sent = *self.bytes_sent.read().await;
        let received = *self.bytes_received.read().await;
        (sent, received)
    }
}

/// Connection manager handles all client connections
pub struct ConnectionManager {
    clients: Arc<RwLock<HashMap<String, Arc<ClientConnection>>>>,
    auth_tokens: Vec<String>,
}

impl ConnectionManager {
    pub fn new(auth_tokens: Vec<String>) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            auth_tokens,
        }
    }

    pub async fn add_client(&self, client: Arc<ClientConnection>) {
        let mut clients = self.clients.write().await;
        clients.insert(client.id.clone(), client);
    }

    pub async fn remove_client(&self, client_id: &str) -> Option<Arc<ClientConnection>> {
        let mut clients = self.clients.write().await;
        clients.remove(client_id)
    }

    pub async fn get_client(&self, client_id: &str) -> Option<Arc<ClientConnection>> {
        let clients = self.clients.read().await;
        clients.get(client_id).cloned()
    }

    pub async fn authenticate(&self, token: &str, client_id: &str) -> bool {
        if !self.auth_tokens.contains(&token.to_string()) {
            warn!(
                "Authentication failed for client {}: invalid token",
                client_id
            );
            return false;
        }

        info!("Client {} authenticated successfully", client_id);
        true
    }

    pub async fn broadcast_message(&self, message: Message) {
        let clients = self.clients.read().await;
        for client in clients.values() {
            if let Err(e) = client.send_message(message.clone()).await {
                error!("Failed to broadcast message to client {}: {}", client.id, e);
            }
        }
    }

    pub async fn get_all_clients(&self) -> Vec<Arc<ClientConnection>> {
        let clients = self.clients.read().await;
        clients.values().cloned().collect()
    }

    pub async fn get_client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
}
