use std::process::Command;

//use crate::error_catcher::ChildProperties;
mod error_catcher;

pub fn status_service(service: Vec<String>) {
    let mut child = Command::new("systemctl")
        .args(["status", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
    child.wait().expect("Failed to Wait child");
}

pub fn reload_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "reload-or-restart", &service[0]])
        .status()
        .expect("Failed to run systemctl command");
}
pub fn enable_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "enable", &service[0]])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn disable_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "disable", &service[0]])
        .status()
        .expect("Failed to run systemctl command");
}
pub fn mask_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "mask", &service[0]])
        .status()
        .expect("Failed to run systemctl command");
}

pub fn unmask_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "unmask", &service[0]])
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
    println!("Usage: systemctl [command] [service]");
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
pub fn start_service(service: &Vec<String>) -> Result<Vec<String>, Vec<String>> {
    // "child" is the needed execution command
    Command::new("sudo")
        .args(["systemctl", "start", &service[0]])
        .output()
        .expect("Failed to run systemctl command");
    error_catcher::start_validation(service)
}

pub fn stop_service(service: &Vec<String>) -> Result<Vec<String>, Vec<String>> {
    Command::new("sudo")
        .args(["systemctl", "stop", &service[0]])
        .output()
        .expect("Failed to run systemctl command");
    error_catcher::stop_validation(service)
}
