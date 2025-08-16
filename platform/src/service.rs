use anyhow::Result;
use std::path::PathBuf;

/// Cross-platform service management
pub trait ServiceManager {
    /// Install the service
    fn install(&self, config: &ServiceConfig) -> Result<()>;

    /// Uninstall the service
    fn uninstall(&self, service_name: &str) -> Result<()>;

    /// Start the service
    fn start(&self, service_name: &str) -> Result<()>;

    /// Stop the service
    fn stop(&self, service_name: &str) -> Result<()>;

    /// Check if service is running
    fn is_running(&self, service_name: &str) -> Result<bool>;
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub user: Option<String>,
}

/// Get the appropriate service manager for the current platform
pub fn get_service_manager() -> Box<dyn ServiceManager> {
    #[cfg(windows)]
    return Box::new(crate::windows::WindowsServiceManager::new());

    #[cfg(unix)]
    return Box::new(crate::linux::LinuxServiceManager::new());
}

/// Check if we're running with sufficient privileges
pub fn has_service_privileges() -> bool {
    #[cfg(windows)]
    {
        // On Windows, check if running as administrator
        crate::windows::is_elevated()
    }

    #[cfg(unix)]
    {
        // On Unix, check if running as root
        unsafe { libc::getuid() == 0 }
    }
}
