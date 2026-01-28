//Start failures:
// Failure cases:
// - Service has configuration errors
// - Dependencies missing or failed
// - Port already in use
// - Insufficient permissions
// - Binary/executable doesn't exist
// - Service crashes immediately after starting
// - Resource limits exceeded (file descriptors, memory)

let result = Command::new("sudo")
    .args(["systemctl", "start", service])
    .output()?;

// Even if exit code is 0, service might fail after

//Stop Failures
// Failure cases:
// - Service refuses to stop (ignoring signals)
// - Timeout waiting for service to stop
// - Service is already stopped (not really a failure)
// - Permission denied
// - Service is in "failed" state (can't stop what's not running)

let result = Command::new("sudo")
    .args(["systemctl", "stop", service])
    .output()?;

if !result.status.success() {
    // Might need to use "kill" instead
    Command::new("sudo")
        .args(["systemctl", "kill", service])
        .output()?;
}

//Enable Failures
// Failure cases:
// - Unit file doesn't exist
// - Unit file has no [Install] section
// - Symlink creation failed (filesystem issues)
// - Permission denied
// - Unit is masked (blocked from being enabled)

let result = Command::new("sudo")
    .args(["systemctl", "enable", service])
    .output()?;

if !result.status.success() {
    let stderr = String::from_utf8_lossy(&result.stderr);
    if stderr.contains("masked") {
        // Need to unmask first
        Command::new("sudo")
            .args(["systemctl", "unmask", service])
            .output()?;
    }
}

//Disable Failures
// Failure cases:
// - Service not enabled (nothing to disable)
// - Permission denied
// - Symlink removal failed

let result = Command::new("sudo")
    .args(["systemctl", "disable", service])
    .output()?;

// Usually doesn't fail, but check exit code


//Restart and Reload Failures
//Failure cases(restart):
// - All STOP failures +
// - All START failures
// - Service stops but won't start again

let result = Command::new("sudo")
    .args(["systemctl", "restart", service])
    .output()?;

// Must check both stop and start phases
// Failure cases(reload):
// - Service doesn't support reload
// - Service not running (can't reload stopped service)
// - Configuration file has errors
// - Service doesn't respond to reload signal

let result = Command::new("sudo")
    .args(["systemctl", "reload", service])
    .output()?;

if !result.status.success() {
    // Fall back to restart
    Command::new("sudo")
        .args(["systemctl", "restart", service])
        .output()?;
}

//Mask Failures
// Failure cases:
// - Permission denied
// - Service already masked

Command::new("sudo")
    .args(["systemctl", "mask", service])
    .output()?;

//Daemon Reload Failures
// Failure cases:
// - Unit file syntax errors
// - Circular dependencies
// - Invalid configuration
// - Permission issues

let result = Command::new("sudo")
    .args(["systemctl", "daemon-reload"])
    .output()?;

if !result.status.success() {
    eprintln!("Configuration errors detected!");
}
