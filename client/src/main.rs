mod config;
mod connection;
mod core;
#[cfg(feature = "gui")]
mod gui;

use clap::Parser;
use config::*;
#[cfg(feature = "gui")]
use gui::NatClientApp;
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
    let config = match load_client_config(&args) {
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

    info!("Starting NAT Traversal Client");

    #[cfg(feature = "gui")]
    let should_use_gui = config.gui.enabled && !args.no_gui;
    #[cfg(not(feature = "gui"))]
    let should_use_gui = false;

    if should_use_gui {
        #[cfg(feature = "gui")]
        {
            // Run GUI application
            let native_options = eframe::NativeOptions {
                initial_window_size: Some([800.0, 600.0].into()),
                min_window_size: Some([600.0, 400.0].into()),
                ..Default::default()
            };

            let mut app = NatClientApp::new(config);

            // Initialize the app asynchronously
            if let Err(e) = app.initialize().await {
                error!("Failed to initialize client: {}", e);
                std::process::exit(1);
            }

            // Run the GUI
            if let Err(e) = eframe::run_native(
                "NAT Traversal Client",
                native_options,
                Box::new(|_cc| Box::new(app)),
            ) {
                error!("Failed to run GUI: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Run CLI application
        info!("Running in CLI mode");

        let client = match core::NatClient::new(config).await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create client: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = client.start().await {
            error!("Client error: {}", e);
            std::process::exit(1);
        }

        // Wait for Ctrl+C
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        info!("Shutting down...");

        if let Err(e) = client.stop().await {
            error!("Error stopping client: {}", e);
        }
    }
}
