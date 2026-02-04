use std::process::Command;

mod error_catcher;

/// Helper: Extract service name from arguments
fn get_service_name(args: &Option<Vec<String>>) -> Option<&str> {
    args.as_ref().and_then(|a| {
        if !a.is_empty() {
            Some(a[0].as_str())
        } else {
            None
        }
    })
}

pub fn status_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    let mut child = Command::new("systemctl")
        .args(["status", service])
        .spawn()
        .expect("Failed to spawn systemctl command");
    child.wait().expect("Failed to Wait child");
}

pub fn reload_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", service])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn enable_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    Command::new("sudo")
        .args(["systemctl", "enable", service])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn disable_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    Command::new("sudo")
        .args(["systemctl", "disable", service])
        .status()
        .expect("Failed to run systemctl command");
}

// Currently, Just these which catch services errors

pub fn mask_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

    // Check if service exists BEFORE attempting unmask
    let check_output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check service");

    let state = String::from_utf8_lossy(&check_output.stdout)
        .trim()
        .to_string();

    // If service doesn't exist, return error early
    if state == "not-found" || (!check_output.status.success() && state.is_empty()) {
        return Err(vec!["Service doesn't exist".to_string()]);
    }

    // No need to check if service is masked, as masking will succeed regardless
    // If service exists, proceed with masking
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

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
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

    // Check if service exists BEFORE attempting unmask
    let check_output = Command::new("systemctl")
        .args(["is-enabled", service])
        .output()
        .expect("Failed to check service");

    let state = String::from_utf8_lossy(&check_output.stdout)
        .trim()
        .to_string();

    // If service doesn't exist, return error early
    if state == "not-found" || (!check_output.status.success() && state.is_empty()) {
        return Err(vec!["Service doesn't exist".to_string()]);
    }

    // If service exists, proceed with unmasking
    // No need to check if service is unmasked, as unmasking will succeed regardless
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

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
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

    Command::new("sudo")
        .args(["systemctl", "start", service])
        .output()
        .expect("Failed to run systemctl command");

    error_catcher::start_validation(args.as_ref().unwrap())
}

pub fn stop_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    let Some(service) = get_service_name(args) else {
        return Err(vec!["No service name provided".to_string()]);
    };

    Command::new("sudo")
        .args(["systemctl", "stop", service])
        .output()
        .expect("Failed to run systemctl command");

    error_catcher::stop_validation(args.as_ref().unwrap())
}

// No Argument Functions:

pub fn list_services() {
    let mut child = Command::new("sh")
        .args(["-c", "systemctl list-units --type=service"])
        .spawn()
        .expect("Failed to spawn systemctl command");
    child.wait().expect("Failed to wait for systemctl command");
    println!("batata command completed!")
}

pub fn help_service() {
    println!("if the command is specified to a service, the <service_name> is required");
    println!("Commands:");
    println!("  start     Start a service");
    println!("  stop      Stop a service");
    println!("  reload    Reload or restart a service");
    println!("  enable    Enable a service to start at boot");
    println!("  disable   Disable a service from starting at boot");
    println!("  reset     Reset the failed services");
    println!("  mask      Prevent a service from being started");
    println!("  unmask    Allow a service to be started");
    println!("  list      List all services");
    println!("  status    Show the status of a service");
}

pub fn reset_service() {
    Command::new("sudo")
        .args(["systemctl", "reset-failed"])
        .status()
        .expect("Failed to run systemctl command");
    println!(
        "Add a report for which services were reset - Lazy Coder!\nBtw, Reset was done successfully..."
    );
}
