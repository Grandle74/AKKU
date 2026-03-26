// modules/services/src/lib.rs
use std::process::{Command, Stdio};

pub mod state_helpers;
mod systemctl_queries;

/// Returns raw `systemctl status` output lines for the given service.
pub fn status_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("systemctl")
        .args(["status", service])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect())
}

pub fn reload_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let _ = Command::new("sudo")
        .args(["systemctl", "reload-or-restart", service])
        .stdout(Stdio::null()) // discard stdout
        .stderr(Stdio::null()) // discard stderr
        .status(); // ignore result; errors handled by your own validation
    systemctl_queries::start_validation(service).map_err(|e| e.join("\n"))
}

pub fn disable_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "disable", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    if output.status.success() {
        systemctl_queries::enable_disable_validation(service, false)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(vec![stderr])
    }
}

pub fn enable_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "enable", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    if output.status.success() {
        systemctl_queries::enable_disable_validation(service, true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(vec![stderr])
    }
}

pub fn mask_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "mask", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    // systemd writes informational messages to stderr even on success.
    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    if output.status.success() {
        let mut result = systemctl_queries::mask_validation(service, true)?;
        result.extend(stderr_lines);
        Ok(result)
    } else {
        systemctl_queries::mask_validation(service, true)
    }
}

pub fn unmask_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "unmask", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    if output.status.success() {
        let mut result = systemctl_queries::mask_validation(service, false)?;
        result.extend(stderr_lines);
        Ok(result)
    } else {
        systemctl_queries::mask_validation(service, false)
    }
}

pub fn start_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    Command::new("sudo")
        .args(["systemctl", "start", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    systemctl_queries::start_validation(service)
}

pub fn stop_service(service: &str) -> Result<Vec<String>, Vec<String>> {
    systemctl_queries::validate_service_exists(service)?;

    Command::new("sudo")
        .args(["systemctl", "stop", service])
        .output()
        .map_err(|e| vec![e.to_string()])?;

    systemctl_queries::stop_validation(service)
}

// ── No-Argument Actions ───────────────────────────────────────────────────────

pub struct ServiceEntry {
    pub name: String,
    pub load_state: String,
    pub active: String,
    pub sub_state: String,
    pub description: String,
}

pub fn list_services() -> Result<Vec<ServiceEntry>, String> {
    let output = Command::new("systemctl")
        .args(["list-units", "--type=service", "--no-pager", "--no-legend"])
        .output()
        .map_err(|e| e.to_string())?;

    let entries = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some(ServiceEntry {
                name: parts.next()?.to_string(),
                load_state: parts.next()?.to_string(),
                active: parts.next()?.to_string(),
                sub_state: parts.next()?.to_string(),
                description: parts.collect::<Vec<_>>().join(" "),
            })
        })
        .collect();

    Ok(entries)
}

pub fn help_service() -> Vec<String> {
    vec![
        "Usage: service <action> [target]".to_string(),
        "       service config <target> <property>=<value> ...".to_string(),
        "".to_string(),
        "Imperative actions:".to_string(),
        "  list              List all services".to_string(),
        "  reset             Reset failed services".to_string(),
        "  status  <name>    Show service status".to_string(),
        "  reload  <name>    Reload or restart a service".to_string(),
        "".to_string(),
        "Declarative (config/change):".to_string(),
        "  service config <name> running=true enabled=yes masked=0".to_string(),
    ]
}

pub fn reset_service() -> Result<Vec<String>, String> {
    let failed_output = Command::new("systemctl")
        .args(["list-units", "--failed", "--no-legend", "--plain"])
        .output()
        .expect("Failed to get failed services");

    let failed_services: Vec<String> = String::from_utf8_lossy(&failed_output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(|s| s.to_string()))
        .collect();

    Command::new("sudo")
        .args(["systemctl", "reset-failed"])
        .status()
        .map_err(|e| e.to_string())?;

    if failed_services.is_empty() {
        Ok(vec!["No failed services to reset".to_string()])
    } else {
        let mut output = Vec::new();

        output.push("Reset the following failed services:".to_string());
        output.extend(failed_services);

        Ok(output)
    }
}
