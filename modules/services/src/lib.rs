use std::process::Command;

//use crate::error_catcher::ChildProperties;
mod error_catcher;

pub fn status_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    let mut child = Command::new("systemctl")
        .args(["status", &args[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
    child.wait().expect("Failed to Wait child");
}

pub fn reload_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", &args[0]])
        .status()
        .expect("Failed to run systemctl command");
}
pub fn enable_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    Command::new("sudo")
        .args(["systemctl", "enable", &args[0]])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn disable_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    Command::new("sudo")
        .args(["systemctl", "disable", &args[0]])
        .status()
        .expect("Failed to run systemctl command");
}
pub fn mask_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    Command::new("sudo")
        .args(["systemctl", "mask", &args[0]])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn unmask_service(args: Option<Vec<String>>) {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            println!("✗ No service name provided");
            return;
        }
    };
    Command::new("sudo")
        .args(["systemctl", "unmask", &args[0]])
        .status()
        .expect("Failed to run systemctl command");
}

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
    // Add a report for which services were reset
    println!(
        "Add a report for which services were reset - Lazy Coder!\nBtw, Reset was done successfully..."
    );
}

// In case of starting failure, this function returns an error message
pub fn start_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            return Err(vec!["No service Action provided".to_string()]);
        }
    };
    // "child" is the needed execution command
    Command::new("sudo")
        .args(["systemctl", "start", &args[0]])
        .output()
        .expect("Failed to run systemctl command");
    error_catcher::start_validation(&args)
}

pub fn stop_service(args: &Option<Vec<String>>) -> Result<Vec<String>, Vec<String>> {
    let args = match args {
        Some(s) if !s.is_empty() => s,
        _ => {
            return Err(vec!["No service Action provided".to_string()]);
        }
    };
    Command::new("sudo")
        .args(["systemctl", "stop", &args[0]])
        .output()
        .expect("Failed to run systemctl command");
    error_catcher::stop_validation(&args)
}
