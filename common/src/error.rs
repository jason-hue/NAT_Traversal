use thiserror::Error;

/// Common error types for the NAT traversal system
#[derive(Error, Debug)]
pub enum NatError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Tunnel error: {message}")]
    Tunnel { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Protocol error: {message}")]
    Protocol { message: String },

    #[error("Connection error: {message}")]
    Connection { message: String },

    #[error("Timeout error: {message}")]
    Timeout { message: String },

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

impl NatError {
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    pub fn tunnel(message: impl Into<String>) -> Self {
        Self::Tunnel {
            message: message.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol {
            message: message.into(),
        }
    }

    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(std::io::Error::new(
            std::io::ErrorKind::Other,
            message.into(),
        ))
    }

    pub fn tls(message: impl Into<String>) -> Self {
        Self::Tls(rustls::Error::General(message.into()))
    }
}

/// Result type alias for NAT traversal operations
pub type NatResult<T> = Result<T, NatError>;
