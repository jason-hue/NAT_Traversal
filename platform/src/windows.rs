#[cfg(windows)]
use crate::service::{ServiceConfig, ServiceManager};
#[cfg(windows)]
use anyhow::Result;
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(windows)]
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager as WinServiceManager, ServiceManagerAccess},
};

#[cfg(windows)]
pub struct WindowsServiceManager;

#[cfg(windows)]
impl WindowsServiceManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl ServiceManager for WindowsServiceManager {
    fn install(&self, config: &ServiceConfig) -> Result<()> {
        let manager = WinServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
        )?;

        let service_info = ServiceInfo {
            name: OsString::from(&config.name),
            display_name: OsString::from(&config.display_name),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: config.executable_path.clone(),
            launch_arguments: config.arguments.iter().map(|s| OsString::from(s)).collect(),
            dependencies: vec![],
            account_name: config.user.as_ref().map(|u| OsString::from(u)),
            account_password: None,
        };

        let _service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;

        tracing::info!("Service '{}' installed successfully", config.name);
        Ok(())
    }

    fn uninstall(&self, service_name: &str) -> Result<()> {
        let manager =
            WinServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

        let service = manager.open_service(service_name, ServiceAccess::DELETE)?;
        service.delete()?;

        tracing::info!("Service '{}' uninstalled successfully", service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> Result<()> {
        let manager =
            WinServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

        let service = manager.open_service(service_name, ServiceAccess::START)?;
        service.start(&[] as &[&str])?;

        tracing::info!("Service '{}' started successfully", service_name);
        Ok(())
    }

    fn stop(&self, service_name: &str) -> Result<()> {
        let manager =
            WinServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

        let service = manager.open_service(service_name, ServiceAccess::STOP)?;
        service.stop()?;

        tracing::info!("Service '{}' stopped successfully", service_name);
        Ok(())
    }

    fn is_running(&self, service_name: &str) -> Result<bool> {
        let manager =
            WinServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;

        let service = manager.open_service(service_name, ServiceAccess::QUERY_STATUS)?;
        let status = service.query_status()?;

        Ok(status.current_state == windows_service::service::ServiceState::Running)
    }
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};

    unsafe {
        let mut token_handle = std::ptr::null_mut();
        if winapi::um::processthreadsapi::OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_QUERY,
            &mut token_handle,
        ) == 0
        {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut return_length = 0;

        if GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        ) == 0
        {
            winapi::um::handleapi::CloseHandle(token_handle);
            return false;
        }

        winapi::um::handleapi::CloseHandle(token_handle);
        elevation.TokenIsElevated != 0
    }
}
