//use crate::error_catcher::validate_service_exists; <- sadly its return value cannot be used here
use std::process::Command;

pub struct ServiceState {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub masked: bool,
}

impl ServiceState {
    pub fn new(service_name: &str) -> Result<ServiceState, String> {
        if !ServiceState::check_exists(service_name) {
            return Err(format!("Service '{}' does not exist", service_name));
        }
        Ok(ServiceState {
            name: service_name.to_string(),
            active: ServiceState::check_active(service_name), // ← Module does systemctl
            enabled: ServiceState::check_enabled(service_name), // ← Module does systemctl
            masked: ServiceState::check_masked(service_name), // ← Module does systemctl
        })
    }

    fn check_exists(service: &str) -> bool {
        let output = Command::new("systemctl")
            .args(["cat", service]) // "cat" shows unit file
            .output()
            .expect("Failed to check service");

        output.status.success()
    }

    fn check_active(service: &str) -> bool {
        let output = Command::new("systemctl")
            .args(["is-active", service])
            .output()
            .expect("Failed to check active state");

        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        state == "active"
    }

    fn check_enabled(service: &str) -> bool {
        let output = Command::new("systemctl")
            .args(["is-enabled", service])
            .output()
            .expect("Failed to check enabled state");

        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        state == "enabled"
    }

    fn check_masked(service: &str) -> bool {
        let output = Command::new("systemctl")
            .args(["is-enabled", service])
            .output()
            .expect("Failed to check enabled state");

        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        state == "masked"
    }
}
