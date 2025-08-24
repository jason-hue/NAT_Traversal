use crate::connection::ConnectionManager;
use chrono::Utc;
use nat_traversal_common::{
    error::{NatError, NatResult},
    protocol::{Message, TunnelInfo, TunnelProtocol},
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Manages tunnels and port forwarding
pub struct TunnelManager {
    tunnels: Arc<RwLock<HashMap<Uuid, TunnelHandler>>>,
    port_allocator: Arc<RwLock<PortAllocator>>,
    connection_manager: Arc<ConnectionManager>,
}

/// Handles a specific tunnel
pub struct TunnelHandler {
    pub info: TunnelInfo,
    pub listener: Option<TcpListener>,
    pub client_id: String,
    pub connections: Arc<RwLock<HashMap<u32, TunnelConnection>>>,
    pub next_connection_id: Arc<RwLock<u32>>,
}

/// Represents a connection through a tunnel
pub struct TunnelConnection {
    pub id: u32,
    pub client_addr: SocketAddr,
    pub sender: mpsc::UnboundedSender<Vec<u8>>,
}

/// Manages port allocation for tunnels
pub struct PortAllocator {
    allocated_ports: HashMap<u16, Uuid>,
    next_port: u16,
    port_range: (u16, u16),
}

impl PortAllocator {
    pub fn new(port_range: (u16, u16)) -> Self {
        Self {
            allocated_ports: HashMap::new(),
            next_port: port_range.0,
            port_range,
        }
    }

    pub fn allocate_port(&mut self, preferred_port: Option<u16>) -> Option<u16> {
        // Try preferred port first
        if let Some(port) = preferred_port {
            if port >= self.port_range.0
                && port <= self.port_range.1
                && !self.allocated_ports.contains_key(&port)
            {
                self.allocated_ports.insert(port, Uuid::nil()); // Temporary placeholder
                return Some(port);
            }
        }

        // Find next available port
        let start_port = self.next_port;
        loop {
            if !self.allocated_ports.contains_key(&self.next_port) {
                let port = self.next_port;
                self.next_port += 1;
                if self.next_port > self.port_range.1 {
                    self.next_port = self.port_range.0;
                }
                return Some(port);
            }

            self.next_port += 1;
            if self.next_port > self.port_range.1 {
                self.next_port = self.port_range.0;
            }

            // Prevent infinite loop
            if self.next_port == start_port {
                break;
            }
        }

        None
    }


    pub fn release_port(&mut self, port: u16) -> bool {
        self.allocated_ports.remove(&port).is_some()
    }
}

impl TunnelManager {
    pub fn new(connection_manager: Arc<ConnectionManager>, port_range: (u16, u16)) -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            port_allocator: Arc::new(RwLock::new(PortAllocator::new(port_range))),
            connection_manager,
        }
    }

    pub async fn create_tunnel(
        &self,
        client_id: String,
        local_port: u16,
        remote_port: Option<u16>,
        protocol: TunnelProtocol,
        name: Option<String>,
    ) -> NatResult<TunnelInfo> {
        let tunnel_id = Uuid::new_v4();

        // Allocate remote port
        let mut allocator = self.port_allocator.write().await;
        let assigned_port = allocator
            .allocate_port(remote_port)
            .ok_or_else(|| NatError::tunnel("No available ports"))?;

        // Update the reservation with the actual tunnel ID
        allocator.allocated_ports.insert(assigned_port, tunnel_id);
        drop(allocator);

        // Create tunnel info
        let tunnel_info = TunnelInfo {
            id: tunnel_id,
            name,
            protocol,
            local_port,
            remote_port: assigned_port,
            created_at: Utc::now(),
            bytes_sent: 0,
            bytes_received: 0,
            active_connections: 0,
        };

        // Create tunnel handler
        let tunnel_handler = TunnelHandler {
            info: tunnel_info.clone(),
            listener: None,
            client_id: client_id.clone(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            next_connection_id: Arc::new(RwLock::new(1)),
        };

        // Store tunnel
        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(tunnel_id, tunnel_handler);
        drop(tunnels);

        // Start listening for connections
        self.start_tunnel_listener(tunnel_id).await?;

        info!(
            "Created tunnel {} for client {} - {}:{} -> {}:{}",
            tunnel_id, client_id, assigned_port, protocol, local_port, protocol
        );

        Ok(tunnel_info)
    }

    pub async fn close_tunnel(&self, tunnel_id: &Uuid) -> NatResult<()> {
        let mut tunnels = self.tunnels.write().await;
        if let Some(tunnel) = tunnels.remove(tunnel_id) {
            // Release port
            let mut allocator = self.port_allocator.write().await;
            allocator.release_port(tunnel.info.remote_port);

            info!("Closed tunnel {}", tunnel_id);
            Ok(())
        } else {
            Err(NatError::tunnel("Tunnel not found"))
        }
    }

    async fn start_tunnel_listener(&self, tunnel_id: Uuid) -> NatResult<()> {
        let tunnels = self.tunnels.clone();
        let connection_manager = self.connection_manager.clone();

        tokio::spawn(async move {
            let (listener, client_id, _protocol, port) = {
                let mut tunnels_guard = tunnels.write().await;
                let tunnel = tunnels_guard.get_mut(&tunnel_id).unwrap();

                let bind_addr = format!("0.0.0.0:{}", tunnel.info.remote_port);
                let listener = match TcpListener::bind(&bind_addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        error!("Failed to bind to {}: {}", bind_addr, e);
                        return;
                    }
                };

                // Note: tokio TcpListener doesn't have try_clone, we'll store the bind address instead
                (
                    listener,
                    tunnel.client_id.clone(),
                    tunnel.info.protocol,
                    tunnel.info.remote_port,
                )
            };

            info!("Tunnel {} listening on port {}", tunnel_id, port);

            // Accept connections
            while let Ok((stream, addr)) = listener.accept().await {
                let tunnels = tunnels.clone();
                let connection_manager = connection_manager.clone();
                let client_id = client_id.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::handle_tunnel_connection(
                        tunnel_id,
                        stream,
                        addr,
                        tunnels,
                        connection_manager,
                        client_id,
                    )
                    .await
                    {
                        error!("Error handling tunnel connection: {}", e);
                    }
                });
            }
        });

        Ok(())
    }

    async fn handle_tunnel_connection(
        tunnel_id: Uuid,
        mut stream: TcpStream,
        client_addr: SocketAddr,
        tunnels: Arc<RwLock<HashMap<Uuid, TunnelHandler>>>,
        connection_manager: Arc<ConnectionManager>,
        client_id: String,
    ) -> NatResult<()> {
        // Get next connection ID
        let connection_id = {
            let tunnels_guard = tunnels.read().await;
            let tunnel = tunnels_guard.get(&tunnel_id).unwrap();
            let mut next_id = tunnel.next_connection_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        debug!(
            "New connection {} to tunnel {} from {}",
            connection_id, tunnel_id, client_addr
        );

        // Notify client about new connection
        if let Some(client) = connection_manager.get_client(&client_id).await {
            let message = Message::NewConnection {
                tunnel_id,
                connection_id,
                client_addr,
            };

            if let Err(e) = client.send_message(message).await {
                error!("Failed to notify client about new connection: {}", e);
                return Err(e);
            }
        }

        // Handle data forwarding
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Store connection
        {
            let tunnels_guard = tunnels.read().await;
            let tunnel = tunnels_guard.get(&tunnel_id).unwrap();
            let mut connections = tunnel.connections.write().await;
            connections.insert(
                connection_id,
                TunnelConnection {
                    id: connection_id,
                    client_addr,
                    sender: tx,
                },
            );
        }

        // Split stream for reading and writing
        let (mut reader, mut writer) = tokio::io::split(stream);

        // Read from TCP connection and forward to client
        let tunnels_read = tunnels.clone();
        let connection_manager_read = connection_manager.clone();
        let client_id_read = client_id.clone();

        tokio::spawn(async move {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        let data = buffer[..n].to_vec();

                        // Send data to client
                        if let Some(client) =
                            connection_manager_read.get_client(&client_id_read).await
                        {
                            let message = Message::Data {
                                tunnel_id,
                                data,
                                connection_id,
                            };

                            if let Err(e) = client.send_message(message).await {
                                error!("Failed to forward data to client: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading from connection: {}", e);
                        break;
                    }
                }
            }

            // Clean up connection
            let tunnels_guard = tunnels_read.read().await;
            if let Some(tunnel) = tunnels_guard.get(&tunnel_id) {
                let mut connections = tunnel.connections.write().await;
                connections.remove(&connection_id);
            }
        });

        // Write data from client to TCP connection
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                if let Err(e) = writer.write_all(&data).await {
                    error!("Error writing to connection: {}", e);
                    break;
                }
            }
        });

        Ok(())
    }

    pub async fn forward_data(
        &self,
        tunnel_id: &Uuid,
        connection_id: u32,
        data: Vec<u8>,
    ) -> NatResult<()> {
        let tunnels = self.tunnels.read().await;
        if let Some(tunnel) = tunnels.get(tunnel_id) {
            let connections = tunnel.connections.read().await;
            if let Some(connection) = connections.get(&connection_id) {
                connection
                    .sender
                    .send(data)
                    .map_err(|_| NatError::connection("Failed to forward data"))?;
                return Ok(());
            }
        }

        Err(NatError::tunnel("Connection not found"))
    }

    pub async fn get_tunnel(&self, tunnel_id: &Uuid) -> Option<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(tunnel_id).map(|t| t.info.clone())
    }

    pub async fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().map(|t| t.info.clone()).collect()
    }
}
