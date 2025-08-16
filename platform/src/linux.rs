#[cfg(unix)]
use crate::service::{ServiceConfig, ServiceManager};
#[cfg(unix)]
use anyhow::{anyhow, Result};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
pub struct LinuxServiceManager;

#[cfg(unix)]
impl LinuxServiceManager {
    pub fn new() -> Self {
        Self
    }

    fn systemd_service_path(service_name: &str) -> String {
        format!("/etc/systemd/system/{}.service", service_name)
    }

    fn create_systemd_service_file(config: &ServiceConfig) -> String {
        let working_dir = config
            .working_directory
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| "/".into());

        let args = if config.arguments.is_empty() {
            String::new()
        } else {
            format!(" {}", config.arguments.join(" "))
        };

        format!(
            r#"[Unit]
Description={}
After=network.target

[Service]
Type=simple
User={}
WorkingDirectory={}
ExecStart={}{}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
            config.description,
            config.user.as_ref().unwrap_or(&"root".to_string()),
            working_dir,
            config.executable_path.to_string_lossy(),
            args
        )
    }
}

#[cfg(unix)]
impl ServiceManager for LinuxServiceManager {
    fn install(&self, config: &ServiceConfig) -> Result<()> {
        let service_path = Self::systemd_service_path(&config.name);
        let service_content = Self::create_systemd_service_file(config);

        // Write service file
        fs::write(&service_path, service_content)?;

        // Reload systemd
        let output = Command::new("systemctl")
            .args(&["daemon-reload"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to reload systemd: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Enable service
        let output = Command::new("systemctl")
            .args(&["enable", &config.name])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to enable service: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        tracing::info!("Service '{}' installed successfully", config.name);
        Ok(())
    }

    fn uninstall(&self, service_name: &str) -> Result<()> {
        // Stop service first
        let _ = self.stop(service_name);

        // Disable service
        let output = Command::new("systemctl")
            .args(&["disable", service_name])
            .output()?;

        if !output.status.success() {
            tracing::warn!(
                "Failed to disable service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Remove service file
        let service_path = Self::systemd_service_path(service_name);
        if Path::new(&service_path).exists() {
            fs::remove_file(&service_path)?;
        }

        // Reload systemd
        let output = Command::new("systemctl")
            .args(&["daemon-reload"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to reload systemd: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        tracing::info!("Service '{}' uninstalled successfully", service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> Result<()> {
        let output = Command::new("systemctl")
            .args(&["start", service_name])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to start service: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        tracing::info!("Service '{}' started successfully", service_name);
        Ok(())
    }

    fn stop(&self, service_name: &str) -> Result<()> {
        let output = Command::new("systemctl")
            .args(&["stop", service_name])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to stop service: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        tracing::info!("Service '{}' stopped successfully", service_name);
        Ok(())
    }

    fn is_running(&self, service_name: &str) -> Result<bool> {
        let output = Command::new("systemctl")
            .args(&["is-active", "--quiet", service_name])
            .output()?;

        Ok(output.status.success())
    }
}
