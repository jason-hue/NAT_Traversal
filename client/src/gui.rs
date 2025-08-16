use crate::{connection::ConnectionState, core::NatClient};
use eframe::egui;
use nat_traversal_common::{
    config::{save_config, ClientConfig},
    protocol::{TunnelInfo, TunnelProtocol},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Main application state for the GUI
pub struct NatClientApp {
    client: Option<Arc<NatClient>>,
    config: ClientConfig,

    // UI state
    connection_state: ConnectionState,
    tunnels: Vec<TunnelInfo>,

    // Forms and inputs
    new_tunnel_form: NewTunnelForm,
    settings_window: bool,
    about_window: bool,

    // Async state management
    state_receiver: Option<mpsc::UnboundedReceiver<AppState>>,
    state_sender: Option<mpsc::UnboundedSender<AppState>>,
}

#[derive(Debug, Clone)]
enum AppState {
    ConnectionState(ConnectionState),
    Tunnels(Vec<TunnelInfo>),
}

#[derive(Default)]
struct NewTunnelForm {
    name: String,
    local_port: String,
    remote_port: String,
    protocol: TunnelProtocol,
    auto_start: bool,
}

impl Default for NatClientApp {
    fn default() -> Self {
        let (state_sender, state_receiver) = mpsc::unbounded_channel();
        Self {
            client: None,
            config: ClientConfig::default(),
            connection_state: ConnectionState::Disconnected,
            tunnels: Vec::new(),
            new_tunnel_form: NewTunnelForm::default(),
            settings_window: false,
            about_window: false,
            state_receiver: Some(state_receiver),
            state_sender: Some(state_sender),
        }
    }
}

impl NatClientApp {
    pub fn new(config: ClientConfig) -> Self {
        let (state_sender, state_receiver) = mpsc::unbounded_channel();
        Self {
            config,
            state_receiver: Some(state_receiver),
            state_sender: Some(state_sender),
            ..Default::default()
        }
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        let client = Arc::new(NatClient::new(self.config.clone()).await?);
        self.client = Some(client.clone());

        // Start background task to update state
        if let Some(sender) = &self.state_sender {
            let sender = sender.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Get connection state
                    let state = client.get_connection_state().await;
                    let _ = sender.send(AppState::ConnectionState(state));

                    // Get tunnels
                    let tunnels = client.get_tunnels().await;
                    let _ = sender.send(AppState::Tunnels(tunnels));
                }
            });
        }

        Ok(())
    }

    fn start_client(&mut self) {
        if let Some(client) = &self.client {
            let client = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.start().await {
                    tracing::error!("Failed to start client: {}", e);
                }
            });
        }
    }

    fn stop_client(&mut self) {
        if let Some(client) = &self.client {
            let client = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.stop().await {
                    tracing::error!("Failed to stop client: {}", e);
                }
            });
        }
    }

    fn update_state(&mut self) {
        // Process any pending state updates from background task
        if let Some(receiver) = &mut self.state_receiver {
            while let Ok(state) = receiver.try_recv() {
                match state {
                    AppState::ConnectionState(new_state) => {
                        self.connection_state = new_state;
                    }
                    AppState::Tunnels(new_tunnels) => {
                        self.tunnels = new_tunnels;
                    }
                }
            }
        }
    }
}

impl eframe::App for NatClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update state from async operations
        self.update_state();

        // Request repaint for real-time updates
        ctx.request_repaint();

        // Main menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.settings_window = true;
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        self.stop_client();
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.about_window = true;
                    }
                });
            });
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status_text = match &self.connection_state {
                    ConnectionState::Disconnected => "❌ Disconnected",
                    ConnectionState::Connecting => "🔄 Connecting...",
                    ConnectionState::Connected => "🟡 Connected",
                    ConnectionState::Authenticated => "✅ Authenticated",
                    ConnectionState::Error(msg) => &format!("❌ Error: {}", msg),
                };

                ui.label(status_text);
                ui.separator();
                ui.label(format!("Tunnels: {}", self.tunnels.len()));
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("NAT Traversal Client");

            // Connection controls
            ui.separator();
            ui.horizontal(|ui| {
                let is_connected = matches!(
                    self.connection_state,
                    ConnectionState::Connected | ConnectionState::Authenticated
                );

                if ui
                    .button(if is_connected {
                        "Disconnect"
                    } else {
                        "Connect"
                    })
                    .clicked()
                {
                    if is_connected {
                        self.stop_client();
                    } else {
                        self.start_client();
                    }
                }

                ui.label(format!(
                    "Server: {}:{}",
                    self.config.server.addr, self.config.server.port
                ));
            });

            ui.separator();

            // Tunnels section
            ui.heading("Tunnels");

            // Tunnel list
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.tunnels.is_empty() {
                    ui.label("No tunnels configured");
                } else {
                    for tunnel in &self.tunnels {
                        ui.horizontal(|ui| {
                            ui.label(tunnel.name.as_ref().unwrap_or(&tunnel.id.to_string()));
                            ui.label(format!(
                                "{}:{} -> {}:{}",
                                tunnel.remote_port,
                                tunnel.protocol,
                                tunnel.local_port,
                                tunnel.protocol
                            ));

                            if ui.button("Close").clicked() {
                                if let Some(client) = &self.client {
                                    let client = client.clone();
                                    let tunnel_id = tunnel.id;
                                    tokio::spawn(async move {
                                        if let Err(e) = client.close_tunnel(tunnel_id).await {
                                            tracing::error!("Failed to close tunnel: {}", e);
                                        }
                                    });
                                }
                            }
                        });
                        ui.separator();
                    }
                }
            });

            ui.separator();

            // New tunnel form
            ui.heading("Create New Tunnel");

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_tunnel_form.name);
            });

            ui.horizontal(|ui| {
                ui.label("Local Port:");
                ui.text_edit_singleline(&mut self.new_tunnel_form.local_port);
            });

            ui.horizontal(|ui| {
                ui.label("Remote Port:");
                ui.text_edit_singleline(&mut self.new_tunnel_form.remote_port);
            });

            ui.horizontal(|ui| {
                ui.label("Protocol:");
                ui.radio_value(
                    &mut self.new_tunnel_form.protocol,
                    TunnelProtocol::Tcp,
                    "TCP",
                );
                ui.radio_value(
                    &mut self.new_tunnel_form.protocol,
                    TunnelProtocol::Udp,
                    "UDP",
                );
            });

            if ui.button("Create Tunnel").clicked() {
                if let (Ok(local_port), Some(client)) =
                    (self.new_tunnel_form.local_port.parse::<u16>(), &self.client)
                {
                    let remote_port = if self.new_tunnel_form.remote_port.is_empty() {
                        None
                    } else {
                        self.new_tunnel_form.remote_port.parse().ok()
                    };

                    let name = if self.new_tunnel_form.name.is_empty() {
                        None
                    } else {
                        Some(self.new_tunnel_form.name.clone())
                    };

                    let client = client.clone();
                    let protocol = self.new_tunnel_form.protocol;

                    tokio::spawn(async move {
                        if let Err(e) = client
                            .create_tunnel(local_port, remote_port, protocol, name)
                            .await
                        {
                            tracing::error!("Failed to create tunnel: {}", e);
                        }
                    });

                    // Clear form
                    self.new_tunnel_form = NewTunnelForm::default();
                }
            }
        });

        // Settings window
        let mut close_settings = false;
        if self.settings_window {
            egui::Window::new("Settings")
                .open(&mut self.settings_window)
                .show(ctx, |ui| {
                    ui.heading("Server Settings");

                    ui.horizontal(|ui| {
                        ui.label("Server Address:");
                        ui.text_edit_singleline(&mut self.config.server.addr);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Server Port:");
                        ui.add(
                            egui::DragValue::new(&mut self.config.server.port)
                                .clamp_range(1..=65535),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Token:");
                        ui.text_edit_singleline(&mut self.config.server.token);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Client ID:");
                        ui.text_edit_singleline(&mut self.config.server.client_id);
                    });

                    ui.checkbox(&mut self.config.server.auto_reconnect, "Auto Reconnect");
                    ui.checkbox(&mut self.config.server.tls_verify, "Verify TLS Certificate");

                    if ui.button("Save").clicked() {
                        if let Err(e) = save_config(&self.config, "client.toml") {
                            tracing::error!("Failed to save config: {}", e);
                        }
                        close_settings = true;
                    }
                });
        }

        if close_settings {
            self.settings_window = false;
        }

        // About window
        if self.about_window {
            egui::Window::new("About")
                .open(&mut self.about_window)
                .show(ctx, |ui| {
                    ui.heading("NAT Traversal Client");
                    ui.label("Version 0.1.0");
                    ui.separator();
                    ui.label("A cross-platform NAT traversal solution written in Rust.");
                    ui.label("Built with egui for the GUI interface.");
                });
        }
    }
}
