use chrono::Utc;
use nat_traversal_common::{
    config::ClientConfig,
    error::{NatError, NatResult},
    protocol::{Message, TunnelInfo, TunnelProtocol, PROTOCOL_VERSION},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_rustls::{rustls, TlsConnector, TlsStream};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type SecureClientStream = TlsStream<TcpStream>;

/// Connection state for the client
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Authenticated,
    Error(String),
}

/// Statistics for client connection
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub reconnect_count: u32,
    pub last_ping_time: Option<chrono::DateTime<Utc>>,
    pub uptime: chrono::Duration,
}

/// Manages the connection to the server
pub struct ServerConnection {
    config: ClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    tunnels: Arc<RwLock<HashMap<Uuid, TunnelInfo>>>,
    stats: Arc<RwLock<ConnectionStats>>,
    message_sender: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    tls_connector: TlsConnector,
}

impl ServerConnection {
    pub async fn new(config: ClientConfig) -> NatResult<Self> {
        let tls_connector = Self::setup_tls(&config).await?;

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            message_sender: Arc::new(Mutex::new(None)),
            tls_connector,
        })
    }

    async fn setup_tls(config: &ClientConfig) -> NatResult<TlsConnector> {
        let mut root_cert_store = rustls::RootCertStore::empty();

        if config.server.tls_verify {
            // Add system certificates
            root_cert_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )
            }));
        } else {
            // For development: accept invalid certificates
            warn!("TLS certificate verification is disabled!");
        }

        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        Ok(TlsConnector::from(Arc::new(tls_config)))
    }

    pub async fn connect(&self) -> NatResult<()> {
        self.set_state(ConnectionState::Connecting).await;

        let server_addr = format!("{}:{}", self.config.server.addr, self.config.server.port);

        // Connect to server
        let tcp_stream = TcpStream::connect(&server_addr).await.map_err(|e| {
            NatError::connection(format!("Failed to connect to {}: {}", server_addr, e))
        })?;

        // Perform TLS handshake
        let server_name = rustls::ServerName::try_from(self.config.server.addr.as_str())
            .map_err(|e| NatError::tls(format!("Invalid server name: {}", e)))?;

        let tls_stream = self
            .tls_connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|e| NatError::tls(format!("TLS handshake failed: {}", e)))?;

        info!("Connected to server: {}", server_addr);
        self.set_state(ConnectionState::Connected).await;

        // Setup message handling
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        *self.message_sender.lock().await = Some(message_tx.clone());

        // Start message handling tasks
        let (read_half, write_half) = tokio::io::split(tls_stream);

        let write_task = {
            let message_rx = message_rx;
            tokio::spawn(async move { Self::handle_write(write_half, message_rx).await })
        };

        let read_task = {
            let state = self.state.clone();
            let tunnels = self.tunnels.clone();
            let stats = self.stats.clone();
            tokio::spawn(async move { Self::handle_read(read_half, state, tunnels, stats).await })
        };

        // Authenticate
        self.authenticate().await?;

        // Start heartbeat
        let heartbeat_task = {
            let message_tx = message_tx.clone();
            tokio::spawn(async move { Self::heartbeat_loop(message_tx).await })
        };

        // Wait for any task to complete (indicating disconnection)
        tokio::select! {
            _ = write_task => {},
            _ = read_task => {},
            _ = heartbeat_task => {},
        }

        self.set_state(ConnectionState::Disconnected).await;
        *self.message_sender.lock().await = None;

        Ok(())
    }

    async fn authenticate(&self) -> NatResult<()> {
        let auth_message = Message::Auth {
            version: PROTOCOL_VERSION,
            token: self.config.server.token.clone(),
            client_id: self.config.server.client_id.clone(),
        };

        self.send_message(auth_message).await?;

        // Wait for authentication response
        // Note: In a real implementation, you'd want to wait for the actual response
        // For now, we'll assume success after sending
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.set_state(ConnectionState::Authenticated).await;

        info!("Authenticated with server");
        Ok(())
    }

    async fn handle_write(
        mut writer: tokio::io::WriteHalf<tokio_rustls::client::TlsStream<TcpStream>>,
        mut message_rx: mpsc::UnboundedReceiver<Message>,
    ) -> NatResult<()> {
        use tokio::io::AsyncWriteExt;

        while let Some(message) = message_rx.recv().await {
            let data = message.to_bytes()?;
            let len = data.len() as u32;

            // Write length prefix
            writer.write_all(&len.to_be_bytes()).await?;
            // Write message data
            writer.write_all(&data).await?;
            writer.flush().await?;
        }

        Ok(())
    }

    async fn handle_read(
        mut reader: tokio::io::ReadHalf<tokio_rustls::client::TlsStream<TcpStream>>,
        state: Arc<RwLock<ConnectionState>>,
        tunnels: Arc<RwLock<HashMap<Uuid, TunnelInfo>>>,
        stats: Arc<RwLock<ConnectionStats>>,
    ) -> NatResult<()> {
        use tokio::io::AsyncReadExt;

        loop {
            // Read message length
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).await.is_err() {
                break;
            }
            let len = u32::from_be_bytes(len_buf) as usize;

            if len > 1024 * 1024 {
                // 1MB limit
                error!("Message too large: {} bytes", len);
                break;
            }

            // Read message data
            let mut data = vec![0u8; len];
            if reader.read_exact(&mut data).await.is_err() {
                break;
            }

            // Update stats
            {
                let mut stats_guard = stats.write().await;
                stats_guard.bytes_received += len as u64;
            }

            // Parse message
            let message = match Message::from_bytes(&data) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    continue;
                }
            };

            // Handle message
            Self::handle_message(message, &state, &tunnels).await;
        }

        Ok(())
    }

    async fn handle_message(
        message: Message,
        state: &Arc<RwLock<ConnectionState>>,
        tunnels: &Arc<RwLock<HashMap<Uuid, TunnelInfo>>>,
    ) {
        match message {
            Message::AuthResponse {
                success,
                error,
                server_version: _,
            } => {
                if success {
                    *state.write().await = ConnectionState::Authenticated;
                    info!("Authentication successful");
                } else {
                    let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                    *state.write().await = ConnectionState::Error(error_msg.clone());
                    error!("Authentication failed: {}", error_msg);
                }
            }

            Message::TunnelCreated {
                tunnel_id,
                remote_port,
                local_port,
            } => {
                info!(
                    "Tunnel created: {} -> {}:{}",
                    tunnel_id, remote_port, local_port
                );
                // TODO: Start local proxy for this tunnel
            }

            Message::TunnelClosed { tunnel_id, reason } => {
                info!("Tunnel closed: {} - {}", tunnel_id, reason);
                let mut tunnels_guard = tunnels.write().await;
                tunnels_guard.remove(&tunnel_id);
            }

            Message::NewConnection {
                tunnel_id,
                connection_id,
                client_addr,
            } => {
                debug!(
                    "New connection {} to tunnel {} from {}",
                    connection_id, tunnel_id, client_addr
                );
                // TODO: Handle new connection
            }

            Message::Data {
                tunnel_id,
                data,
                connection_id,
            } => {
                debug!(
                    "Received {} bytes for tunnel {} connection {}",
                    data.len(),
                    tunnel_id,
                    connection_id
                );
                // TODO: Forward data to local service
            }

            Message::Pong { timestamp: _ } => {
                debug!("Received pong");
            }

            Message::Error { code, message } => {
                error!("Server error: {:?} - {}", code, message);
            }

            _ => {
                warn!("Unhandled message type: {:?}", message);
            }
        }
    }

    async fn heartbeat_loop(message_tx: mpsc::UnboundedSender<Message>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            let ping = Message::Ping {
                timestamp: Utc::now(),
            };

            if message_tx.send(ping).is_err() {
                break;
            }
        }
    }

    pub async fn send_message(&self, message: Message) -> NatResult<()> {
        let sender = self.message_sender.lock().await;
        if let Some(tx) = sender.as_ref() {
            tx.send(message)
                .map_err(|_| NatError::connection("Failed to send message"))?;
            Ok(())
        } else {
            Err(NatError::connection("Not connected"))
        }
    }

    pub async fn create_tunnel(
        &self,
        local_port: u16,
        remote_port: Option<u16>,
        protocol: TunnelProtocol,
        name: Option<String>,
    ) -> NatResult<()> {
        let message = Message::CreateTunnel {
            local_port,
            remote_port,
            protocol,
            name,
        };

        self.send_message(message).await
    }

    pub async fn close_tunnel(&self, tunnel_id: Uuid) -> NatResult<()> {
        let message = Message::CloseTunnel { tunnel_id };
        self.send_message(message).await
    }

    pub async fn get_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    async fn set_state(&self, state: ConnectionState) {
        *self.state.write().await = state;
    }

    pub async fn get_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }

    pub async fn get_stats(&self) -> ConnectionStats {
        self.stats.read().await.clone()
    }

    pub async fn run_with_reconnect(&self) -> NatResult<()> {
        loop {
            match self.connect().await {
                Ok(_) => {
                    info!("Connection completed normally");
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                    self.set_state(ConnectionState::Error(e.to_string())).await;
                }
            }

            if !self.config.server.auto_reconnect {
                break;
            }

            info!(
                "Reconnecting in {} seconds...",
                self.config.server.reconnect_interval_secs
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.config.server.reconnect_interval_secs,
            ))
            .await;

            // Update reconnect count
            {
                let mut stats = self.stats.write().await;
                stats.reconnect_count += 1;
            }
        }

        Ok(())
    }
}
