use std::process::{Command, Stdio};

use crate::error_catcher::ChildProperties;
mod error_catcher;

// Status service returns [is-active, is-enabled]
pub fn status_service(service: Vec<String>) -> [String; 2] {
    let comm_active = Command::new("systemctl")
        .args(["is-active", &service[0]])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn systemctl command");
    let comm_active = comm_active.wait_with_output().expect("Output Failed");
    let comm_enabled = Command::new("systemctl")
        .stdout(Stdio::piped())
        .args(["is-enabled", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
    let comm_enabled = comm_enabled.wait_with_output().expect("Output Failed");
    return [
        String::from_utf8_lossy(&comm_active.stdout)
            .trim()
            .to_string(),
        String::from_utf8_lossy(&comm_enabled.stdout)
            .trim()
            .to_string(),
    ];
}

pub fn stop_service(service: Vec<String>) {
    Command::new("sudo")
        .args(["systemctl", "stop", &service[0]])
        .status()
        .expect("Failed to run systemctl command");
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
pub fn remove_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["remove", &service[0]])
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
    // println!("  mask      Prevent a service from being started");
    println!("  list      List all services");
    println!("  status    Show the status of a service");
}

// In case of starting failure, this function returns an error message
pub fn start_service(service: Vec<String>) {
    // "child" is the needed execution command
    let mut child = Command::new("sudo")
        .args(["systemctl", "start", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
    child.wait().expect("Failed to Wait child");
    // dbg
    let debg = ChildProperties::new(service[0].clone());

    println!("{:#?}", debg)
}
