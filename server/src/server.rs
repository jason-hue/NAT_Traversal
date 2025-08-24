use crate::{connection::*, tunnel::TunnelManager};
use nat_traversal_common::{
    config::ServerConfig,
    error::{NatError, NatResult},
    protocol::{ErrorCode, Message, PROTOCOL_VERSION},
};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::{rustls, TlsAcceptor};
use tracing::{debug, error, info, warn};

/// Main server structure
pub struct NatServer {
    config: ServerConfig,
    connection_manager: Arc<ConnectionManager>,
    tunnel_manager: Arc<TunnelManager>,
    tls_acceptor: TlsAcceptor,
}

impl NatServer {
    pub async fn new(config: ServerConfig) -> NatResult<Self> {
        // Setup TLS
        let tls_acceptor = Self::setup_tls(&config).await?;

        // Create connection manager
        let connection_manager = Arc::new(ConnectionManager::new(config.auth.tokens.clone()));

        // Create tunnel manager
        let tunnel_manager = Arc::new(TunnelManager::new(
            connection_manager.clone(),
            (8000, 9000), // Port range for tunnels
        ));

        Ok(Self {
            config,
            connection_manager,
            tunnel_manager,
            tls_acceptor,
        })
    }

    async fn setup_tls(config: &ServerConfig) -> NatResult<TlsAcceptor> {
        // Load certificates
        let cert_file = File::open(&config.tls.cert_path)
            .map_err(|e| NatError::config(format!("Failed to open cert file: {}", e)))?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain = certs(&mut cert_reader)
            .map_err(|e| NatError::config(format!("Failed to parse certificates: {}", e)))?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        // Load private key
        let key_file = File::open(&config.tls.key_path)
            .map_err(|e| NatError::config(format!("Failed to open key file: {}", e)))?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| NatError::config(format!("Failed to parse private key: {}", e)))?;

        if keys.is_empty() {
            return Err(NatError::config("No private key found"));
        }

        let private_key = rustls::PrivateKey(keys.remove(0));

        // Configure TLS
        let tls_config = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| NatError::config(format!("Failed to configure TLS: {}", e)))?;

        Ok(TlsAcceptor::from(Arc::new(tls_config)))
    }

    pub async fn run(&self) -> NatResult<()> {
        let bind_addr = format!(
            "{}:{}",
            self.config.network.bind_addr, self.config.network.port
        );
        let listener = TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| NatError::network(format!("Failed to bind to {}: {}", bind_addr, e)))?;

        info!("NAT Traversal Server listening on {}", bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let tls_acceptor = self.tls_acceptor.clone();
                    let connection_manager = self.connection_manager.clone();
                    let tunnel_manager = self.tunnel_manager.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(
                            stream,
                            addr,
                            tls_acceptor,
                            connection_manager,
                            tunnel_manager,
                        )
                        .await
                        {
                            error!("Error handling client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_client(
        stream: TcpStream,
        addr: std::net::SocketAddr,
        tls_acceptor: TlsAcceptor,
        connection_manager: Arc<ConnectionManager>,
        tunnel_manager: Arc<TunnelManager>,
    ) -> NatResult<()> {
        debug!("New connection from {}", addr);

        // Perform TLS handshake
        let tls_stream = tls_acceptor
            .accept(stream)
            .await
            .map_err(|e| NatError::tls(format!("TLS handshake failed: {}", e)))?;

        // Setup message channels
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (read_half, write_half) = tokio::io::split(tls_stream);

        // Handle message sending
        let write_task = tokio::spawn(async move { Self::handle_write(write_half, rx).await });

        // Handle message receiving and processing
        let read_task = tokio::spawn(async move {
            Self::handle_read(
                read_half,
                addr,
                tx.clone(),
                connection_manager,
                tunnel_manager,
            )
            .await
        });

        // Wait for either task to complete
        tokio::select! {
            _ = write_task => {},
            _ = read_task => {},
        }

        debug!("Client {} disconnected", addr);
        Ok(())
    }

    async fn handle_write(
        mut writer: tokio::io::WriteHalf<tokio_rustls::server::TlsStream<TcpStream>>,
        mut rx: mpsc::UnboundedReceiver<Message>,
    ) -> NatResult<()> {
        use tokio::io::AsyncWriteExt;

        while let Some(message) = rx.recv().await {
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
        mut reader: tokio::io::ReadHalf<tokio_rustls::server::TlsStream<TcpStream>>,
        addr: std::net::SocketAddr,
        tx: mpsc::UnboundedSender<Message>,
        connection_manager: Arc<ConnectionManager>,
        tunnel_manager: Arc<TunnelManager>,
    ) -> NatResult<()> {
        use tokio::io::AsyncReadExt;

        let mut client_connection: Option<Arc<ClientConnection>> = None;

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

            // Parse message
            let message = match Message::from_bytes(&data) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    continue;
                }
            };

            // Handle message
            if let Err(e) = Self::handle_message(
                message,
                &mut client_connection,
                addr,
                &tx,
                &connection_manager,
                &tunnel_manager,
            )
            .await
            {
                error!("Error handling message: {}", e);

                // Send error response
                let error_msg = Message::Error {
                    code: ErrorCode::InternalError,
                    message: e.to_string(),
                };
                let _ = tx.send(error_msg);
            }
        }

        // Clean up client connection
        if let Some(client) = &client_connection {
            connection_manager.remove_client(&client.id).await;
        }

        Ok(())
    }

    async fn handle_message(
        message: Message,
        client_connection: &mut Option<Arc<ClientConnection>>,
        addr: std::net::SocketAddr,
        tx: &mpsc::UnboundedSender<Message>,
        connection_manager: &Arc<ConnectionManager>,
        tunnel_manager: &Arc<TunnelManager>,
    ) -> NatResult<()> {
        match message {
            Message::Auth {
                version,
                token,
                client_id,
            } => {
                if version != PROTOCOL_VERSION {
                    let response = Message::AuthResponse {
                        success: false,
                        error: Some("Protocol version mismatch".to_string()),
                        server_version: PROTOCOL_VERSION,
                    };
                    tx.send(response)
                        .map_err(|_| NatError::connection("Failed to send response"))?;
                    return Ok(());
                }

                let success = connection_manager.authenticate(&token, &client_id).await;

                if success {
                    let client =
                        Arc::new(ClientConnection::new(client_id.clone(), addr, tx.clone()));
                    connection_manager.add_client(client.clone()).await;
                    *client_connection = Some(client);
                }

                let response = Message::AuthResponse {
                    success,
                    error: if success {
                        None
                    } else {
                        Some("Authentication failed".to_string())
                    },
                    server_version: PROTOCOL_VERSION,
                };

                tx.send(response)
                    .map_err(|_| NatError::connection("Failed to send response"))?;
            }

            Message::CreateTunnel {
                local_port,
                remote_port,
                protocol,
                name,
            } => {
                if let Some(client) = client_connection {
                    let tunnel_info = tunnel_manager
                        .create_tunnel(client.id.clone(), local_port, remote_port, protocol, name)
                        .await?;

                    client.add_tunnel(tunnel_info.clone()).await;

                    let response = Message::TunnelCreated {
                        tunnel_id: tunnel_info.id,
                        remote_port: tunnel_info.remote_port,
                        local_port: tunnel_info.local_port,
                        protocol: tunnel_info.protocol,
                        name: tunnel_info.name.clone(),
                    };

                    tx.send(response)
                        .map_err(|_| NatError::connection("Failed to send response"))?;
                } else {
                    return Err(NatError::authentication("Not authenticated"));
                }
            }

            Message::CloseTunnel { tunnel_id } => {
                if let Some(client) = client_connection {
                    tunnel_manager.close_tunnel(&tunnel_id).await?;
                    client.remove_tunnel(&tunnel_id).await;

                    let response = Message::TunnelClosed {
                        tunnel_id,
                        reason: "Closed by client".to_string(),
                    };

                    tx.send(response)
                        .map_err(|_| NatError::connection("Failed to send response"))?;
                } else {
                    return Err(NatError::authentication("Not authenticated"));
                }
            }

            Message::Data {
                tunnel_id,
                data,
                connection_id,
            } => {
                tunnel_manager
                    .forward_data(&tunnel_id, connection_id, data)
                    .await?;
            }

            Message::Ping { timestamp } => {
                let response = Message::Pong { timestamp };
                tx.send(response)
                    .map_err(|_| NatError::connection("Failed to send response"))?;
            }

            Message::StatusRequest => {
                if let Some(client) = client_connection {
                    let tunnels = client.list_tunnels().await;
                    let uptime = (chrono::Utc::now() - client.connected_at).num_seconds() as u64;

                    let response = Message::Status {
                        tunnels,
                        connections: 0, // TODO: count active connections
                        uptime,
                    };

                    tx.send(response)
                        .map_err(|_| NatError::connection("Failed to send response"))?;
                }
            }

            _ => {
                warn!("Unhandled message type: {:?}", message);
            }
        }

        Ok(())
    }
}
