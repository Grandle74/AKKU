use std::process::Command;

mod error_catcher;
pub mod state_helpers;

// this function prints, it needs to return instead!
// (will work on it in the future when needed, in shaa' allah)
pub fn status_service(args: Option<Vec<String>>) -> Result<Vec<String>, String> {
    let service = error_catcher::validate_service_name(&args).map_err(|e| e.join("\n"))?;
    error_catcher::validate_service_exists(service).map_err(|e| e.join("\n"))?;

    let output = Command::new("systemctl")
        .args(["status", service])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.to_string())
        .collect())
}

/// Currently, Just these Actions which returns services errors

pub fn reload_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting Action
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", service])
        .status()
        .expect("Failed to run systemctl command");

    error_catcher::start_validation(args.as_ref().unwrap())
}

pub fn disable_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting Action
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "disable", service])
        .output()
        .expect("Failed to run systemctl command");

    if output.status.success() {
        error_catcher::enable_disable_validation(service, false)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(vec![stderr])
    }
}

pub fn enable_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting Action
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "enable", service])
        .output()
        .expect("Failed to run systemctl command");

    if output.status.success() {
        error_catcher::enable_disable_validation(service, true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(vec![stderr])
    }
}

pub fn mask_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting mask...
    // Doing this now prevents unnecessary actions such as masking a non-existent service anyways
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "mask", service])
        .output()
        .expect("Failed to run systemctl command");

    // Get stderr messages (systemd's weird default is stderr!?)
    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    // Check exit code to determine success/failure
    if output.status.success() {
        // Success - stderr contains info messages
        let mut result = error_catcher::mask_validation(service, true)?;
        result.extend(stderr_lines);
        Ok(result)
    } else {
        // Failure - stderr contains error messages
        error_catcher::mask_validation(service, true)
    }
}

pub fn unmask_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting unmask...
    // Doing this now prevents unnecessary actions such as unmasking a non-existent service anyways
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    let output = Command::new("sudo")
        .args(["systemctl", "unmask", service])
        .output()
        .expect("Failed to run systemctl command");

    let stderr_lines: Vec<String> = String::from_utf8_lossy(&output.stderr)
        .trim()
        .lines()
        .map(|s| s.to_string())
        .collect();

    if output.status.success() {
        let mut result = error_catcher::mask_validation(service, false)?;
        result.extend(stderr_lines);
        Ok(result)
    } else {
        error_catcher::mask_validation(service, false)
    }
}

pub fn start_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting Action
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    Command::new("sudo")
        .args(["systemctl", "start", service])
        .output()
        .expect("Failed to run systemctl command");

    error_catcher::start_validation(args.as_ref().unwrap())
}

pub fn stop_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    // Check if exists BEFORE attempting Action
    let service = error_catcher::validate_service_name(&args)?;
    error_catcher::validate_service_exists(service)?;

    Command::new("sudo")
        .args(["systemctl", "stop", service])
        .output()
        .expect("Failed to run systemctl command");

    error_catcher::stop_validation(args.as_ref().unwrap())
}

/// No Argument Actions
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

    let stdout = String::from_utf8_lossy(&output.stdout);

    let entries = stdout
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
        "  reset             Reset the failed services".to_string(),
        "  status  <name>    Show service status".to_string(),
        "".to_string(),
        "Declarative (config/change):".to_string(),
        "  service config <name> running=true enabled=yes masked=0".to_string(),
    ]
}

pub fn reset_service() -> Result<Vec<String>, String> {
    // Get list of failed services BEFORE reset
    let failed_services = Command::new("systemctl")
        .args(["list-units", "--failed", "--no-legend", "--plain"])
        .output()
        .expect("Failed to get failed services");

    let failed_services: Vec<String> = String::from_utf8_lossy(&failed_services.stdout)
        .lines()
        .filter_map(|line| {
            // Extract service name (first column)
            line.split_whitespace().next().map(|s| s.to_string())
        })
        .collect();

    // Run reset command
    Command::new("sudo")
        .args(["systemctl", "reset-failed"])
        .status()
        .expect("Failed to run systemctl command");

    // Return list of reset services
    if failed_services.is_empty() {
        Ok(vec!["No failed services to reset".to_string()])
    } else {
        Ok(failed_services)
    }
}
