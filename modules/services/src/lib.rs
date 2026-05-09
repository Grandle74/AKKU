// modules/services/src/lib.rs
//
// Public API of the Services module.
//
// Each function here is a thin wrapper around one systemctl command.
// Responsibility: execute the command and return either success lines
// or a human-readable error string. NO decision logic lives here —
// that belongs to the engine's planner and executor.
//
// Error convention: ALL functions return `Result<Vec<String>, String>`.
// The executor converts these to `Result<Vec<String>, Vec<String>>` when
// assembling the final EngineResult. Do not deviate from this signature.

use std::process::{Command, Stdio};

pub mod state_helpers;
mod systemctl_queries;

// ── Targeted Actions ─────────────────────────────────────────────────────────

/// Returns raw `systemctl status` output for the named service.
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

/// Reloads (or restarts if reload is not supported) the named service,
/// then validates the resulting state.
pub fn reload_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    // `reload-or-restart` is used intentionally — not all services support reload.
    // We discard stdout/stderr here because `start_validation` will authoritatively
    // confirm the final state via systemd properties. The command's own output is
    // unreliable for state confirmation.
    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", service])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    // NOTE: We intentionally do not check `.status.success()` here. The reload
    // command may exit non-zero even when the service transitions correctly (e.g.
    // a restart that systemd itself handles). `start_validation` is the ground truth.

    systemctl_queries::start_validation(service).map_err(|lines| lines.join("\n"))
}

pub fn start_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "start", service])
        .output()
        .map_err(|e| e.to_string())?;

    systemctl_queries::start_validation(service).map_err(|lines| lines.join("\n"))
}

pub fn stop_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "stop", service])
        .output()
        .map_err(|e| e.to_string())?;

    systemctl_queries::stop_validation(service).map_err(|lines| lines.join("\n"))
}

pub fn enable_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("sudo")
        .args(["systemctl", "enable", service])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        systemctl_queries::enable_disable_validation(service, true)
            .map_err(|lines| lines.join("\n"))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn disable_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("sudo")
        .args(["systemctl", "disable", service])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        systemctl_queries::enable_disable_validation(service, false)
            .map_err(|lines| lines.join("\n"))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn mask_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("sudo")
        .args(["systemctl", "mask", service])
        .output()
        .map_err(|e| e.to_string())?;

    // systemd writes informational messages to stderr even on success for mask/unmask.
    // We collect them to include in the result for transparency.
    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    let mut result =
        systemctl_queries::mask_validation(service, true).map_err(|lines| lines.join("\n"))?;
    result.extend(stderr_lines);
    Ok(result)
}

pub fn unmask_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("sudo")
        .args(["systemctl", "unmask", service])
        .output()
        .map_err(|e| e.to_string())?;

    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    let mut result =
        systemctl_queries::mask_validation(service, false).map_err(|lines| lines.join("\n"))?;
    result.extend(stderr_lines);
    Ok(result)
}

// ── No-Target Actions ─────────────────────────────────────────────────────────

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

pub fn reset_service() -> Result<Vec<String>, String> {
    let failed_output = Command::new("systemctl")
        .args(["list-units", "--failed", "--no-legend", "--plain"])
        .output()
        .map_err(|e| e.to_string())?;

    let failed: Vec<String> = String::from_utf8_lossy(&failed_output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(|s| s.to_string()))
        .collect();

    Command::new("sudo")
        .args(["systemctl", "reset-failed"])
        .status()
        .map_err(|e| e.to_string())?;

    if failed.is_empty() {
        Ok(vec!["No failed services to reset.".to_string()])
    } else {
        let mut out = vec!["Reset the following failed services:".to_string()];
        out.extend(failed);
        Ok(out)
    }
}

const HELP_TEXT: &str = include_str!("../docs/help.txt");
pub fn help_service() -> Vec<String> {
    let mut lines: Vec<String> = HELP_TEXT.lines().map(|s| s.to_string()).collect();

    lines.insert(0, String::new());
    lines.push(String::new());
    lines
}
