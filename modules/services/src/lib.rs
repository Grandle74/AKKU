// modules/services/src/lib.rs
//
// Public API of the Services module.
//
// Does not own state diffing, step ordering, or any decision about *whether*
// a command should run — those live in state_helpers and the engine's executor.
//
// TODO: This module is a prototype placeholder and will be rewritten before v0.1.
// The systemctl wrappers and validation layers have known reliability issues
// across machines. state_helpers.rs is considered stable; the rest is not.

use std::process::{Command, Stdio};

pub mod state_helpers;
mod systemctl_queries;

// ── Targeted Actions ──────────────────────────────────────────────────────────

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

/// Reloads the named service, falling back to restart if reload is unsupported.
///
/// Does not check the command's exit code — `start_validation` is the
/// authoritative ground truth for final state. The command's own output is
/// unreliable for state confirmation.
pub fn reload_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", service])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;

    systemctl_queries::start_validation(service).map_err(|lines| lines.join("\n"))
}

/// Starts the named service and validates the resulting state.
pub fn start_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "start", service])
        .output()
        .map_err(|e| e.to_string())?;

    systemctl_queries::start_validation(service).map_err(|lines| lines.join("\n"))
}

/// Stops the named service and validates the resulting state.
pub fn stop_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "stop", service])
        .output()
        .map_err(|e| e.to_string())?;

    systemctl_queries::stop_validation(service).map_err(|lines| lines.join("\n"))
}

/// Enables the named service and validates the resulting state.
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

/// Disables the named service and validates the resulting state.
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

/// Masks the named service and validates the resulting state.
///
/// systemd writes informational messages to stderr even on success for
/// mask/unmask — these are collected and appended to the result for
/// transparency rather than discarded.
pub fn mask_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("sudo")
        .args(["systemctl", "mask", service])
        .output()
        .map_err(|e| e.to_string())?;

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

/// Unmasks the named service and validates the resulting state.
///
/// See `mask_service` for the stderr collection rationale.
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

/// Clears the failed state of a specific service.
pub fn reset_failed_service(service: &str) -> Result<Vec<String>, String> {
    systemctl_queries::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    Command::new("sudo")
        .args(["systemctl", "reset-failed", service])
        .status()
        .map_err(|e| e.to_string())?;

    Ok(vec![format!("Cleared failed state for '{}'", service)])
}

// ── No-Target Actions ─────────────────────────────────────────────────────────

pub struct ServiceEntry {
    pub name: String,
    pub load_state: String,
    pub active: String,
    pub sub_state: String,
    pub description: String,
}

/// Returns all loaded service units as structured entries.
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

/// Resets ALL failed services system-wide.
///
/// This is a Meta action only — invoked by `service reset` from the CLI.
/// Plans never use this; they call `reset_failed_service(target)` instead.
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

/// Returns the module help text as a vec of lines, padded with blank lines.
pub fn help_service() -> Vec<String> {
    let mut lines: Vec<String> = HELP_TEXT.lines().map(|s| s.to_string()).collect();
    lines.insert(0, String::new());
    lines.push(String::new());
    lines
}
