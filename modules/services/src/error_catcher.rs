use std::process::Command;

#[derive(Debug)]
pub struct ChildProperties {
    // IDK WHAT ARE THESE COMMENTS X)
    pub load_state: String,            // preflight only
    pub active_state: String,          // required
    pub sub_state: String,             // required
    pub result: String,                // required
    pub exec_main_status: Option<i32>, // numeric truth
    pub main_pid: Option<u32>,         // informational
}

// This implementation provides a constructor for ChildProperties and automatic parsing of properties
// Used by start_validation() and stop_validation() functions
impl ChildProperties {
    pub fn new(service: String) -> Self {
        let mut prop = Self {
            load_state: "".to_string(),
            active_state: "".to_string(),
            sub_state: "".to_string(),
            result: "".to_string(),
            main_pid: None,
            exec_main_status: None,
        };

        prop.prop_parser(service);
        return prop;
    }

    fn prop_parser(&mut self, service: String) {
        // This adds a small delay to let systemd update the status -- it's the only solution
        println!("Parsing properties for service: {}...", service);
        std::thread::sleep(std::time::Duration::from_millis(3000));
        // "child_status" is the needed status of the child command to catch error efficiently
        let child_status = Command::new("systemctl")
            .args([
                "show",
                &service,
                //the following are all the needed properties to cover 100% of results
                "--property=LoadState,ExecMainStatus,Result,ActiveState,MainPID,SubState",
            ])
            .output()
            .expect("Failed to check status");

        let child_status = String::from_utf8_lossy(&child_status.stdout)
            .trim()
            .to_string();
        // Collecting a vector of service's properties
        let child_status: Vec<&str> = child_status.lines().map(|s| s).collect();
        // Deviding the Property and its Value into Tuple(Property, Value)
        let child_status: Vec<(&str, &str)> = child_status
            .iter()
            .map(|s| {
                let mut val = s.split("=");
                (val.next().unwrap(), val.next().unwrap())
            })
            .collect();
        // dbg
        // println!("{:?}", child_status);

        // Transforming "child_status" into a Valid Struct "ChildProperties"
        // i.e.: Storing each Property Value with its right Struct Field
        for i in 0..child_status.len() {
            match child_status[i].0 {
                "LoadState" => {
                    self.load_state = child_status[i].1.to_string();
                }
                "ActiveState" => {
                    self.active_state = child_status[i].1.to_string();
                }
                "Result" => {
                    self.result = child_status[i].1.to_string();
                }
                "MainPID" => {
                    self.main_pid = {
                        if child_status[i].1.to_string().parse::<u32>().is_ok() {
                            Some(child_status[i].1.to_string().parse::<u32>().unwrap())
                        } else {
                            None
                        }
                    };
                }
                "ExecMainStatus" => {
                    self.exec_main_status = {
                        if child_status[i].1.to_string().parse::<i32>().is_ok() {
                            Some(child_status[i].1.to_string().parse::<i32>().unwrap())
                        } else {
                            None
                        }
                    };
                }
                "SubState" => {
                    self.sub_state = child_status[i].1.to_string();
                }
                _ => {
                    panic!("No Properties for some reason... go fix your code!")
                }
            }
        }
    }
}

/// Helper: Extract service name from arguments
pub fn get_service_name(args: &Option<Vec<String>>) -> Option<&str> {
    args.as_ref().and_then(|a| {
        if !a.is_empty() {
            Some(a[0].as_str())
        } else {
            None
        }
    })
}

/// Generic validation functions
pub fn validate_service_name(args: &Option<Vec<String>>) -> Result<&str, Vec<String>> {
    match get_service_name(args) {
        Some(name) => Ok(name),
        None => Err(vec!["No service name provided".to_string()]),
    }
}
pub fn validate_service_exists(service: &str) -> Result<(), Vec<String>> {
    let output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check service");

    let state = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if state == "not-found" || (!output.status.success() && state.is_empty()) {
        Err(vec!["Service doesn't exist".to_string()])
    } else {
        Ok(())
    }
}

/// Specific validation functions
pub fn start_validation(service: &Vec<String>) -> Result<Vec<String>, Vec<String>> {
    let props = ChildProperties::new(service[0].clone());
    // dbg
    // println!("{:#?}", props);
    let mut vals: Vec<String> = Vec::new();

    // 1st Layer
    match props.load_state.as_str() {
        "loaded" => {
            vals.push("Service exists and loaded correctly".to_string());
        }
        "masked" => return error(vals, "Service is masked"),
        "error" => return error(vals, "Configuration file has an error"),
        _ => return error(vals, "Service doesn't exist"),
    }

    // Not a Layer
    // Aslong as the service is loaded, the main PID is not None
    vals.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

    // 2nd Layer
    match props.exec_main_status {
        Some(0) => {}
        Some(1) => return error(vals, "Service crashed"),
        Some(126) => return error(vals, "Permission denied"),
        Some(127) => return error(vals, "Executable not found"),
        Some(status) if status >= 128 => {
            return error(
                vals,
                format!("Service crashed via signal {}", status - 128).as_str(),
            );
        }
        _ => {
            if let Some(petipain) = props.exec_main_status {
                return error(vals, format!("Exec Main Status: {}", petipain).as_str());
            }
        }
    }

    // 3rd Layer
    if props.result.as_str() != "success" {
        return Err(vals);
    }

    // 4th Layer
    match props.active_state.as_str() {
        "active" => {
            vals.push("Service is running".to_string());
        }
        "activating" | "deactivating" => return error(vals, "Action stuck - Timeout"),
        _ => return error(vals, "Failed to start service"),
    }

    // 5th Layer
    match props.sub_state.as_str() {
        "running" => {
            //println!("Service is running")
        }
        "exited" => return error(vals, "Service exited with error"),
        "failed" => return error(vals, "Service failed to start"),
        "auto-restart" => return error(vals, "Service crashed"),
        "dead" => return error(vals, "Service died unexpectedly"),
        "inactive" => return error(vals, "Service is inactive"),
        _ => return error(vals, "Service status unknown"),
    }
    // If no Error was detected/catched, it will reach here -> Returning Ok() :)
    return Ok(vals);
}

pub fn stop_validation(service: &Vec<String>) -> Result<Vec<String>, Vec<String>> {
    let props = ChildProperties::new(service[0].clone());
    // dbg
    // println!("{:#?}", props);
    let mut vals: Vec<String> = Vec::new();

    // 1st Layer
    match props.load_state.as_str() {
        "loaded" => {
            vals.push("Service exists and loaded correctly".to_string());
        }
        "masked" => {
            vals.push("Service is masked".to_string());
        }
        "error" => return error(vals, "Configuration file has an error"),
        _ => return error(vals, "Service doesn't exist"),
    }

    // Not a Layer
    // Aslong as the service is loaded, the main PID is not None
    vals.push(match &props.main_pid {
        Some(pid) => format!("Main PID: {}", pid),
        None => "Main PID: None - not running".to_string(),
    });

    // 2nd Layer
    match props.exec_main_status {
        Some(0) => {}
        Some(1) => return error(vals, "Service crashed"),
        Some(126) => return error(vals, "Permission denied"),
        Some(127) => return error(vals, "Executable not found"),
        Some(status) if status >= 128 => {
            return error(
                vals,
                format!("Service crashed via signal {}", status - 128).as_str(),
            );
        }
        _ => {
            if let Some(petipain) = props.exec_main_status {
                return error(vals, format!("Exec Main Status: {}", petipain).as_str());
            }
        }
    }

    // 3rd Layer
    if props.result.as_str() != "success" {
        return Err(vals);
    }

    // 4th Layer
    match props.active_state.as_str() {
        "inactive" => {
            vals.push("Service is inactive".to_string());
        }
        "activating" | "deactivating" => return error(vals, "Action stuck - Timeout"),
        _ => return error(vals, "Failed to stop service"),
    }

    // 5th Layer
    match props.sub_state.as_str() {
        "dead" | "inactive" => {
            // empty since active state tells us that the service is inactive
        }
        "exited" => return error(vals, "Service exited with error"),
        "failed" => return error(vals, "Service failed to stop"),
        "auto-restart" => return error(vals, "Service crashed earlier"),
        "running" => return error(vals, "Service is unexpectedly running"),

        _ => return error(vals, "Service status unknown"),
    }
    // If no Error was detected/catched, it will reach here -> Returning Ok() :)
    return Ok(vals);
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
            "Service is static - cannot be enabled/disabled".to_string(),
        ]),
        "not-found" => Err(vec!["Service doesn't exist".to_string()]),
        _ => Err(vec![format!("Current state: {}", state)]),
    }
}

// Returning Error is used multiple times - this will be used to avoid code duplication
fn error(mut vals: Vec<String>, push_last_message: &str) -> Result<Vec<String>, Vec<String>> {
    vals.push(push_last_message.to_string());
    Err(vals)
}
// Also, Could use a Macro:
/*
macro_rules! fail {
    ($vals:expr, $msg:expr) => {{
        $vals.push($msg.to_string());
        return Err($vals);
    }};
}
*/
