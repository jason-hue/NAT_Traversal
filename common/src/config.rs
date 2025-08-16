use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    pub tls: TlsConfig,
    pub auth: AuthConfig,
    pub limits: LimitsConfig,
    pub logging: LoggingConfig,
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server: ServerConnectionConfig,
    pub tunnels: Vec<TunnelConfig>,
    pub gui: GuiConfig,
    pub logging: LoggingConfig,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bind_addr: IpAddr,
    pub port: u16,
    pub max_connections: u32,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub ca_path: Option<PathBuf>,
    pub verify_client: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub tokens: Vec<String>,
    pub require_auth: bool,
    pub max_clients_per_token: Option<u32>,
}

/// Rate limiting and resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_tunnels_per_client: u32,
    pub max_bandwidth_mbps: Option<u32>,
    pub max_connections_per_tunnel: u32,
    pub connection_timeout_secs: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<PathBuf>,
    pub max_size_mb: u32,
    pub max_files: u32,
}

/// Server connection configuration for client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnectionConfig {
    pub addr: String,
    pub port: u16,
    pub token: String,
    pub client_id: String,
    pub auto_reconnect: bool,
    pub reconnect_interval_secs: u64,
    pub tls_verify: bool,
}

/// Tunnel configuration for client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub name: String,
    pub local_port: u16,
    pub remote_port: Option<u16>,
    pub protocol: crate::protocol::TunnelProtocol,
    pub auto_start: bool,
}

/// GUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub enabled: bool,
    pub start_minimized: bool,
    pub system_tray: bool,
    pub theme: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                bind_addr: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                port: 7000,
                max_connections: 1000,
            },
            tls: TlsConfig {
                cert_path: "server.crt".into(),
                key_path: "server.key".into(),
                ca_path: None,
                verify_client: false,
            },
            auth: AuthConfig {
                tokens: vec!["default-token".to_string()],
                require_auth: true,
                max_clients_per_token: Some(10),
            },
            limits: LimitsConfig {
                max_tunnels_per_client: 10,
                max_bandwidth_mbps: None,
                max_connections_per_tunnel: 100,
                connection_timeout_secs: 300,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
                max_size_mb: 100,
                max_files: 5,
            },
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server: ServerConnectionConfig {
                addr: "localhost".to_string(),
                port: 7000,
                token: "default-token".to_string(),
                client_id: "default-client".to_string(),
                auto_reconnect: true,
                reconnect_interval_secs: 30,
                tls_verify: true,
            },
            tunnels: vec![],
            gui: GuiConfig {
                enabled: true,
                start_minimized: false,
                system_tray: true,
                theme: "dark".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
                max_size_mb: 50,
                max_files: 3,
            },
        }
    }
}

/// Cross-platform configuration paths
pub fn get_config_dir() -> anyhow::Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "nat-traversal", "nat-traversal")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let config_dir = dirs.config_dir();
    std::fs::create_dir_all(config_dir)?;
    Ok(config_dir.to_path_buf())
}

/// Load configuration from file with fallback to default
pub fn load_config<T>(file_name: &str) -> anyhow::Result<T>
where
    T: Default + for<'de> Deserialize<'de>,
{
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join(file_name);

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        Ok(toml::from_str(&content)?)
    } else {
        Ok(T::default())
    }
}

/// Save configuration to file
pub fn save_config<T>(config: &T, file_name: &str) -> anyhow::Result<()>
where
    T: Serialize,
{
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join(file_name);

    let content = toml::to_string_pretty(config)?;
    std::fs::write(&config_path, content)?;
    Ok(())
}
