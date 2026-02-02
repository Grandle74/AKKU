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

pub fn mask_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    Command::new("sudo")
        .args(["systemctl", "mask", service])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn unmask_service(args: Option<Vec<String>>) {
    let Some(service) = get_service_name(&args) else {
        println!("✗ No service name provided");
        return;
    };

    Command::new("sudo")
        .args(["systemctl", "unmask", service])
        .status()
        .expect("Failed to run systemctl command");
}

// Currently, Just these which catch services errors

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
