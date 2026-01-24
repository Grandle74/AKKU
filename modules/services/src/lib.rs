use std::process::{Command, Stdio};

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

pub fn start_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["start", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}

pub fn stop_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["stop", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}
pub fn reload_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["reload-or-restart", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}
pub fn enable_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["enable", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}

pub fn disable_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["disable", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}
pub fn remove_service(service: Vec<String>) {
    Command::new("systemctl")
        .args(["remove", &service[0]])
        .spawn()
        .expect("Failed to spawn systemctl command");
}

pub fn list_services() {
    Command::new("systemctl")
        .args(["list-units", "--type=service"])
        .spawn()
        .expect("Failed to spawn systemctl command");
}

pub fn help_service() {
    println!("Usage: systemctl [command] [service]");
    println!("Commands:");
    println!("  start     Start a service");
    println!("  stop      Stop a service");
    println!("  reload    Reload a service");
    println!("  enable    Enable a service");
    println!("  disable   Disable a service");
    println!("  remove    Remove a service");
}
