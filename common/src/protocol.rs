use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

/// Message types exchanged between client and server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Authentication request from client
    Auth {
        version: u32,
        token: String,
        client_id: String,
    },

    /// Authentication response from server
    AuthResponse {
        success: bool,
        error: Option<String>,
        server_version: u32,
    },

    /// Create a new tunnel
    CreateTunnel {
        local_port: u16,
        remote_port: Option<u16>, // None for auto-assign
        protocol: TunnelProtocol,
        name: Option<String>,
    },

    /// Tunnel creation response
    TunnelCreated {
        tunnel_id: Uuid,
        remote_port: u16,
        local_port: u16,
        protocol: TunnelProtocol,
        name: Option<String>,
    },

    /// Close an existing tunnel
    CloseTunnel { tunnel_id: Uuid },

    /// Tunnel closed notification
    TunnelClosed { tunnel_id: Uuid, reason: String },

    /// Data transfer through tunnel
    Data {
        tunnel_id: Uuid,
        data: Vec<u8>,
        connection_id: u32,
    },

    /// New connection to tunneled service
    NewConnection {
        tunnel_id: Uuid,
        connection_id: u32,
        client_addr: SocketAddr,
    },

    /// Connection closed
    ConnectionClosed { tunnel_id: Uuid, connection_id: u32 },

    /// Heartbeat ping
    Ping { timestamp: DateTime<Utc> },

    /// Heartbeat pong response
    Pong { timestamp: DateTime<Utc> },

    /// Status request
    StatusRequest,

    /// Status response with current tunnels
    Status {
        tunnels: Vec<TunnelInfo>,
        connections: u32,
        uptime: u64, // seconds
    },

    /// Error message
    Error { code: ErrorCode, message: String },
}

/// Supported tunnel protocols
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TunnelProtocol {
    #[default]
    Tcp,
    Udp,
}

/// Tunnel information for status reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub id: Uuid,
    pub name: Option<String>,
    pub protocol: TunnelProtocol,
    pub local_port: u16,
    pub remote_port: u16,
    pub created_at: DateTime<Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: u32,
}

/// Error codes for protocol errors
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorCode {
    AuthenticationFailed,
    InvalidMessage,
    TunnelNotFound,
    PortInUse,
    PermissionDenied,
    RateLimitExceeded,
    InternalError,
    ProtocolVersionMismatch,
}

impl Message {
    /// Serialize message to binary format
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize message from binary format
    pub fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }
}

impl std::fmt::Display for TunnelProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunnelProtocol::Tcp => write!(f, "TCP"),
            TunnelProtocol::Udp => write!(f, "UDP"),
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::AuthenticationFailed => write!(f, "Authentication failed"),
            ErrorCode::InvalidMessage => write!(f, "Invalid message"),
            ErrorCode::TunnelNotFound => write!(f, "Tunnel not found"),
            ErrorCode::PortInUse => write!(f, "Port already in use"),
            ErrorCode::PermissionDenied => write!(f, "Permission denied"),
            ErrorCode::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ErrorCode::InternalError => write!(f, "Internal server error"),
            ErrorCode::ProtocolVersionMismatch => write!(f, "Protocol version mismatch"),
        }
    }
}
