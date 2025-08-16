use clap::Parser;
use nat_traversal_common::config::{load_config, save_config, ClientConfig};
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "nat-client")]
#[command(about = "NAT Traversal Client")]
pub struct Args {
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Server address
    #[arg(short, long)]
    pub server: Option<String>,

    /// Server port
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Client token
    #[arg(short, long)]
    pub token: Option<String>,

    /// Generate default configuration file
    #[arg(long)]
    pub generate_config: bool,

    /// Run without GUI (CLI mode)
    #[arg(long)]
    pub no_gui: bool,

    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn load_client_config(args: &Args) -> anyhow::Result<ClientConfig> {
    let mut config: ClientConfig = if let Some(config_path) = &args.config {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        load_config("client.toml")?
    };

    // Override with command line arguments
    if let Some(server) = &args.server {
        config.server.addr = server.clone();
    }

    if let Some(port) = args.port {
        config.server.port = port;
    }

    if let Some(token) = &args.token {
        config.server.token = token.clone();
    }

    if args.no_gui {
        config.gui.enabled = false;
    }

    if args.verbose {
        config.logging.level = "debug".to_string();
    }

    Ok(config)
}

pub fn generate_default_config() -> anyhow::Result<()> {
    let config = ClientConfig::default();
    save_config(&config, "client.toml")?;
    info!("Generated default configuration file: client.toml");
    Ok(())
}

pub fn setup_logging(config: &ClientConfig) -> anyhow::Result<()> {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true);

    if let Some(log_file) = &config.logging.file {
        let file_appender = tracing_appender::rolling::daily(
            log_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
            log_file
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("client.log")),
        );
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}
