mod config;
mod connection;
mod server;
mod tunnel;

use clap::Parser;
use config::*;
use server::NatServer;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Generate config if requested
    if args.generate_config {
        if let Err(e) = generate_default_config() {
            eprintln!("Failed to generate config: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Load configuration
    let config = match load_server_config(&args) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Setup logging
    if let Err(e) = setup_logging(&config) {
        eprintln!("Failed to setup logging: {}", e);
        std::process::exit(1);
    }

    info!("Starting NAT Traversal Server");

    // Create and run server
    let server = match NatServer::new(config).await {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to create server: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
