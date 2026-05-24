// modules/services/src/systemctl_queries.rs
//
// Raw systemd queries and post-action validation for the Services module.
//
// Does not decide what to run or in what order — it only confirms whether
// a systemd state transition landed where it was expected to.

use std::process::Command;

// ── Service Properties ────────────────────────────────────────────────────────

/// Properties queried from systemd after a start or stop command.
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

    fn parse_systemctl_output(&mut self, text: &str) {
        for line in text.lines() {
            let mut parts = line.splitn(2, '=');
            let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };
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

    fn query_systemctl(service: &str) -> String {
        let bytes = Command::new("systemctl")
            .args([
                "show",
                service,
                "--property=LoadState,ExecMainStatus,Result,ActiveState,\
                 MainPID,SubState,ActiveEnterTimestampMonotonic,\
                 InactiveEnterTimestampMonotonic",
            ])
            .output()
            .expect("Failed to query systemctl properties")
            .stdout;
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn query_and_parse(&mut self, service: String) {
        // Poll until systemd is no longer mid-transition, then read final state.
        // 150 ms × 20 attempts = 3 s maximum wait.
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(150);
        const MAX_ATTEMPTS: usize = 20;

        for attempt in 0..MAX_ATTEMPTS {
            self.parse_systemctl_output(&Self::query_systemctl(&service));

            let still_transitioning = matches!(
                self.active_state.as_str(),
                "activating" | "deactivating" | "reloading"
            );

            if !still_transitioning {
                if self.active_state == "active" {
                    // Type=simple services can appear "active" for a moment before
                    // crashing. Wait briefly, then recheck.
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    self.parse_systemctl_output(&Self::query_systemctl(&service));

                    // A newer inactive timestamp means the service crashed after
                    // the active snapshot was taken.
                    if self.inactive_enter_timestamp > self.active_enter_timestamp
                        && self.inactive_enter_timestamp != 0
                    {
                        self.active_state = "failed".to_string();
                        self.sub_state = "dead".to_string();
                    }
                }
                return;
            }

            if attempt < MAX_ATTEMPTS - 1 {
                println!(
                    "Waiting for service to settle... ({}/{})",
                    attempt + 1,
                    MAX_ATTEMPTS
                );
                std::thread::sleep(POLL_INTERVAL);
            }
        }
        // State never settled — validation layers will reject "activating".
    }
}

// ── Existence Check ───────────────────────────────────────────────────────────

/// Returns an error if the service is not known to systemd.
///
/// Uses LoadState rather than the command exit code — `systemctl show` exits 0
/// even for unknown units.
pub fn validate_service_exists(service: &str) -> Result<(), Vec<String>> {
    let output = Command::new("systemctl")
        .args(["show", service, "--property=LoadState"])
        .output()
        .expect("Failed to query systemctl");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let load_state = stdout
        .lines()
        .find_map(|line| line.strip_prefix("LoadState="))
        .unwrap_or("");

    if load_state == "not-found" || load_state.is_empty() {
        Err(vec![format!("Service '{}' doesn't exist", service)])
    } else {
        Ok(())
    }
}

// ── Post-Action Validation ────────────────────────────────────────────────────

/// Verifies that the service is running after a start or reload command.
pub fn start_validation(service: &str) -> Result<Vec<String>, Vec<String>> {
    let props = ServiceProperties::new(service.to_string());
    let mut messages: Vec<String> = Vec::new();

    match props.load_state.as_str() {
        "loaded" => messages.push("Service exists and loaded correctly".to_string()),
        "masked" => return fail(messages, "Service is masked"),
        "error" => return fail(messages, "Configuration file has an error"),
        _ => return fail(messages, &format!("Service '{}' doesn't exist", service)),
    }

    messages.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

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

    if props.result.as_str() != "success" {
        return Err(messages);
    }

    match props.active_state.as_str() {
        "active" => messages.push("Service is running".to_string()),
        "activating" | "deactivating" => return fail(messages, "Action stuck - Timeout"),
        _ => return fail(messages, "Failed to start service"),
    }

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

    match props.load_state.as_str() {
        "loaded" => messages.push("Service exists and loaded correctly".to_string()),
        // Masked services are already "stopped" by definition.
        "masked" => messages.push("Service is masked".to_string()),
        "error" => return fail(messages, "Configuration file has an error"),
        _ => return fail(messages, &format!("Service '{}' doesn't exist", service)),
    }

    messages.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

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

    if props.result.as_str() != "success" {
        return Err(messages);
    }

    match props.active_state.as_str() {
        "inactive" => messages.push("Service is inactive".to_string()),
        "activating" | "deactivating" => return fail(messages, "Action stuck - Timeout"),
        _ => return fail(messages, "Failed to stop service"),
    }

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

/// Verifies that the service's mask state matches `expect_masked`.
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

/// Verifies that the service's enable state matches `expect_enabled`.
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
        // "static" units have no [Install] section — enable/disable is a no-op.
        "static" => Err(vec![
            "Service is static — cannot be enabled/disabled".to_string(),
        ]),
        "not-found" => Err(vec![format!("Service '{}' doesn't exist", service)]),
        _ => Err(vec![format!("Unexpected state: {}", state)]),
    }
}

// ── Internal Helper ───────────────────────────────────────────────────────────

/// Appends `reason` to `messages` and returns the vec as `Err`.
fn fail(mut messages: Vec<String>, reason: &str) -> Result<Vec<String>, Vec<String>> {
    messages.push(reason.to_string());
    Err(messages)
}
