// modules/services/src/error_catcher.rs
use std::process::Command;

/// Properties queried from systemd after running a start/stop command.
/// Used by `start_validation()` and `stop_validation()` to confirm the outcome.
#[derive(Debug)]
pub struct ServiceProperties {
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub result: String,
    pub exec_main_status: Option<i32>,
    pub main_pid: Option<u32>,
    pub active_enter_timestamp: u64,
    pub inactive_enter_timestamp: u64,
}

impl ServiceProperties {
    pub fn new(service: String) -> Self {
        let mut props = Self {
            load_state: String::new(),
            active_state: String::new(),
            sub_state: String::new(),
            result: String::new(),
            main_pid: None,
            exec_main_status: None,
            active_enter_timestamp: 0,
            inactive_enter_timestamp: 0,
        };
        props.query_and_parse(service);
        props
    }

    fn query_and_parse(&mut self, service: String) {
        // Poll until systemd is no longer mid-transition, then read final state.
        // Checks every 150ms, gives up after 3 seconds total (20 attempts).
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(150);
        const MAX_ATTEMPTS: usize = 20;

        for attempt in 0..MAX_ATTEMPTS {
            let raw_output = Command::new("systemctl")
                .args([
                    "show",
                    &service,
                    "--property=LoadState,ExecMainStatus,Result,ActiveState,MainPID,SubState,ActiveEnterTimestampMonotonic,InactiveEnterTimestampMonotonic",
                ])
                .output()
                .expect("Failed to query systemctl properties");

            let raw_text = String::from_utf8_lossy(&raw_output.stdout)
                .trim()
                .to_string();

            let kv_pairs: Vec<(&str, &str)> = raw_text
                .lines()
                .filter_map(|line| {
                    let mut parts = line.splitn(2, '=');
                    Some((parts.next()?, parts.next()?))
                })
                .collect();

            // Parse into self first
            for (key, value) in &kv_pairs {
                match *key {
                    "LoadState" => self.load_state = value.to_string(),
                    "ActiveState" => self.active_state = value.to_string(),
                    "Result" => self.result = value.to_string(),
                    "SubState" => self.sub_state = value.to_string(),
                    "MainPID" => self.main_pid = value.parse().ok(),
                    "ExecMainStatus" => self.exec_main_status = value.parse().ok(),
                    "ActiveEnterTimestampMonotonic" => {
                        self.active_enter_timestamp = value.parse().unwrap_or(0)
                    }
                    "InactiveEnterTimestampMonotonic" => {
                        self.inactive_enter_timestamp = value.parse().unwrap_or(0)
                    }
                    _ => {}
                }
            }

            // If systemd is done transitioning, we have the real state — stop polling.
            let still_transitioning = matches!(
                self.active_state.as_str(),
                "activating" | "deactivating" | "reloading"
            );

            if !still_transitioning {
                // For active services: confirm the state holds briefly.
                // Catches Type=simple services that start then immediately crash.
                if self.active_state == "active" {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    // One re-query — overwrite self with confirmed final state.
                    let recheck = Command::new("systemctl")
                        .args([
                            "show", &service,
                            "--property=LoadState,ExecMainStatus,Result,ActiveState,MainPID,SubState,ActiveEnterTimestampMonotonic,InactiveEnterTimestampMonotonic",
                        ])
                        .output()
                        .expect("Failed to re-query systemctl properties");

                    for line in String::from_utf8_lossy(&recheck.stdout).lines() {
                        let mut parts = line.splitn(2, '=');
                        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                            match key {
                                "LoadState" => self.load_state = value.to_string(),
                                "ActiveState" => self.active_state = value.to_string(),
                                "Result" => self.result = value.to_string(),
                                "SubState" => self.sub_state = value.to_string(),
                                "MainPID" => self.main_pid = value.parse().ok(),
                                "ExecMainStatus" => self.exec_main_status = value.parse().ok(),
                                "ActiveEnterTimestampMonotonic" => {
                                    self.active_enter_timestamp = value.parse().unwrap_or(0)
                                }
                                "InactiveEnterTimestampMonotonic" => {
                                    self.inactive_enter_timestamp = value.parse().unwrap_or(0)
                                }
                                _ => {}
                            }
                        }
                    }

                    // Now the timestamp check is meaningful — crash has had time to register.
                    if self.inactive_enter_timestamp > self.active_enter_timestamp
                        && self.inactive_enter_timestamp != 0
                    {
                        self.active_state = "failed".to_string();
                        self.sub_state = "dead".to_string();
                    }
                }
                return;
            }

            // Still transitioning — wait and retry, unless this was the last attempt.
            if attempt < MAX_ATTEMPTS - 1 {
                println!(
                    "Waiting for service to settle... ({}/{})",
                    attempt + 1,
                    MAX_ATTEMPTS
                );
                std::thread::sleep(POLL_INTERVAL);
            }
        }
        // Fell through — state never settled. Validation layers will catch "activating" and fail.
    }
}

/// Returns an error if the service does not exist on this system.
pub fn validate_service_exists(service: &str) -> Result<(), Vec<String>> {
    let output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check service");

    let state = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if state == "not-found" || (!output.status.success() && state.is_empty()) {
        Err(vec![
            format!("Service '{}' doesn't exist", service)
                .as_str()
                .to_string(),
        ])
    } else {
        Ok(())
    }
}

// ── Post-Action Validation ────────────────────────────────────────────────────

/// Verifies that the service is running after a start/reload command.
pub fn start_validation(service: &str) -> Result<Vec<String>, Vec<String>> {
    let props = ServiceProperties::new(service.to_string());
    let mut messages: Vec<String> = Vec::new();

    // Layer 1: Service must be loaded.
    match props.load_state.as_str() {
        "loaded" => messages.push("Service exists and loaded correctly".to_string()),
        "masked" => return fail(messages, "Service is masked"),
        "error" => return fail(messages, "Configuration file has an error"),
        _ => {
            return fail(
                messages,
                format!("Service '{}' doesn't exist", service).as_str(),
            );
        }
    }

    messages.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

    // Layer 2: Check exit code of the main process.
    match props.exec_main_status {
        Some(0) => {}
        Some(1) => return fail(messages, "Service crashed"),
        Some(126) => return fail(messages, "Permission denied"),
        Some(127) => return fail(messages, "Executable not found"),
        Some(status) if status >= 128 => {
            return fail(
                messages,
                &format!("Service crashed via signal {}", status - 128),
            );
        }
        Some(status) => return fail(messages, &format!("Unexpected ExecMainStatus: {}", status)),
        None => {}
    }

    // Layer 3: Overall result must be "success".
    if props.result.as_str() != "success" {
        return Err(messages);
    }

    // Layer 4: Active state.
    match props.active_state.as_str() {
        "active" => messages.push("Service is running".to_string()),
        "activating" | "deactivating" => return fail(messages, "Action stuck - Timeout"),
        _ => return fail(messages, "Failed to start service"),
    }

    // Layer 5: Sub-state.
    match props.sub_state.as_str() {
        "running" => {}
        "exited" => return fail(messages, "Service exited with error"),
        "failed" => return fail(messages, "Service failed to start"),
        "auto-restart" => return fail(messages, "Service crashed"),
        "dead" => return fail(messages, "Service died unexpectedly"),
        "inactive" => return fail(messages, "Service is inactive"),
        _ => return fail(messages, "Service status unknown"),
    }

    Ok(messages)
}

/// Verifies that the service is stopped after a stop command.
pub fn stop_validation(service: &str) -> Result<Vec<String>, Vec<String>> {
    let props = ServiceProperties::new(service.to_string());
    let mut messages: Vec<String> = Vec::new();

    // Layer 1: Service must be loaded (or masked — masked services can be "stopped").
    match props.load_state.as_str() {
        "loaded" => messages.push("Service exists and loaded correctly".to_string()),
        "masked" => messages.push("Service is masked".to_string()),
        "error" => return fail(messages, "Configuration file has an error"),
        _ => {
            return fail(
                messages,
                format!("Service '{}' doesn't exist", service).as_str(),
            );
        }
    }

    messages.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

    // Layer 2: Check exit code.
    match props.exec_main_status {
        Some(0) => {}
        Some(1) => return fail(messages, "Service crashed"),
        Some(126) => return fail(messages, "Permission denied"),
        Some(127) => return fail(messages, "Executable not found"),
        Some(status) if status >= 128 => {
            return fail(
                messages,
                &format!("Service crashed via signal {}", status - 128),
            );
        }
        Some(status) => return fail(messages, &format!("Unexpected ExecMainStatus: {}", status)),
        None => {}
    }

    // Layer 3: Overall result must be "success".
    if props.result.as_str() != "success" {
        return Err(messages);
    }

    // Layer 4: Active state must be "inactive".
    match props.active_state.as_str() {
        "inactive" => messages.push("Service is inactive".to_string()),
        "activating" | "deactivating" => return fail(messages, "Action stuck - Timeout"),
        _ => return fail(messages, "Failed to stop service"),
    }

    // Layer 5: Sub-state.
    match props.sub_state.as_str() {
        "dead" | "inactive" => {}
        "exited" => return fail(messages, "Service exited with error"),
        "failed" => return fail(messages, "Service failed to stop"),
        "auto-restart" => return fail(messages, "Service crashed earlier"),
        "running" => return fail(messages, "Service is unexpectedly running"),
        _ => return fail(messages, "Service status unknown"),
    }

    Ok(messages)
}

pub fn mask_validation(service: &str, expect_masked: bool) -> Result<Vec<String>, Vec<String>> {
    let output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check status");

    let state = String::from_utf8(output.stdout)
        .expect("Failed to convert output to string")
        .trim()
        .to_string();

    let is_masked = state == "masked";

    if is_masked == expect_masked {
        Ok(vec![format!("Current state: {}", state)])
    } else {
        Err(vec![if expect_masked {
            format!("Masking failed, current state: {}", state)
        } else {
            "Still masked".to_string()
        }])
    }
}

pub fn enable_disable_validation(
    service: &str,
    expect_enabled: bool,
) -> Result<Vec<String>, Vec<String>> {
    let output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check status");

    let state = String::from_utf8(output.stdout)
        .expect("Failed to convert output to string")
        .trim()
        .to_string();

    match state.as_str() {
        "enabled" if expect_enabled => Ok(vec![format!("Current state: {}", state)]),
        "disabled" if !expect_enabled => Ok(vec![format!("Current state: {}", state)]),
        "masked" => Err(vec!["Service is masked".to_string()]),
        "static" => Err(vec![
            "Service is static — cannot be enabled/disabled".to_string(),
        ]),
        "not-found" => Err(vec![
            format!("Service '{}' doesn't exist", service)
                .as_str()
                .to_string(),
        ]),
        _ => Err(vec![format!("Unexpected state: {}", state)]),
    }
}

// ── Internal Helper ───────────────────────────────────────────────────────────

/// Appends a final error message to `messages` and returns it as `Err`.
/// Avoids repeating the push + return pattern throughout validation layers.
fn fail(mut messages: Vec<String>, reason: &str) -> Result<Vec<String>, Vec<String>> {
    messages.push(reason.to_string());
    Err(messages)
}
