use std::process::{Command, Stdio};

pub fn service_status(service: &str) -> [String; 2] {
    let comm_active = Command::new("systemctl")
        .args(["is-active", service])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn systemctl command");
    let comm_active = comm_active.wait_with_output().expect("Output Failed");
    let comm_enabled = Command::new("systemctl")
        .stdout(Stdio::piped())
        .args(["is-enabled", service])
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
