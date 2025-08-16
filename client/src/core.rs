use crate::connection::{ConnectionState, ServerConnection};
use nat_traversal_common::{
    config::ClientConfig,
    protocol::{TunnelInfo, TunnelProtocol},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Core client functionality
pub struct NatClient {
    config: ClientConfig,
    connection: Arc<ServerConnection>,
    running: Arc<RwLock<bool>>,
}

impl NatClient {
    pub async fn new(config: ClientConfig) -> anyhow::Result<Self> {
        let connection = Arc::new(ServerConnection::new(config.clone()).await?);

        Ok(Self {
            config,
            connection,
            running: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        *self.running.write().await = true;

        // Start connection with auto-reconnect
        let connection = self.connection.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                if let Err(e) = connection.run_with_reconnect().await {
                    tracing::error!("Connection error: {}", e);
                    break;
                }
            }
        });

        // Start configured tunnels
        for tunnel_config in &self.config.tunnels {
            if tunnel_config.auto_start {
                if let Err(e) = self
                    .create_tunnel(
                        tunnel_config.local_port,
                        tunnel_config.remote_port,
                        tunnel_config.protocol,
                        Some(tunnel_config.name.clone()),
                    )
                    .await
                {
                    tracing::warn!("Failed to start tunnel {}: {}", tunnel_config.name, e);
                }
            }
        }

        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        *self.running.write().await = false;
        Ok(())
    }

    pub async fn create_tunnel(
        &self,
        local_port: u16,
        remote_port: Option<u16>,
        protocol: TunnelProtocol,
        name: Option<String>,
    ) -> anyhow::Result<()> {
        self.connection
            .create_tunnel(local_port, remote_port, protocol, name)
            .await?;
        Ok(())
    }

    pub async fn close_tunnel(&self, tunnel_id: Uuid) -> anyhow::Result<()> {
        self.connection.close_tunnel(tunnel_id).await?;
        Ok(())
    }

    pub async fn get_connection_state(&self) -> ConnectionState {
        self.connection.get_state().await
    }

    pub async fn get_tunnels(&self) -> Vec<TunnelInfo> {
        self.connection.get_tunnels().await
    }

    pub fn get_config(&self) -> &ClientConfig {
        &self.config
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}
