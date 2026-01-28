use std::process::Output;

#[derive(Debug)]
pub struct ChildProperties {
    // Property: (Value, Fail/Success, Reason)
    load_state: (String, bool, String),
    active_state: (String, bool, String),
    result: (String, bool, String),
    main_pid: (String, bool, String),
    can_start: (String, bool, String),
}
pub fn error_catcher(child_status: Output) -> ChildProperties {
    // Not Ideal Instance, shall work on it later...
    /*
    let mut thee = ChildProperties {
        load_state: (String::new(), false, String::new()),
        active_state: (String::new(), false, String::new()),
        result: (String::new(), false, String::new()),
        main_pid: (String::new(), false, String::new()),
        can_start: (String::new(), false, String::new()),
    };
    */
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
    //dbg
    println!("{:?}", child_status);

    // Transforming the Vector into a Valid Struct "ChildProperties"
    // first declaring a variable of type ChildProperties
    let mut thee_collector: Vec<(String, bool, String)> = [].to_vec();

    // Put this for loop inside the instance of thee_collector
    //
    for i in 0..child_status.len() {
        match child_status[i] {
            ("LoadState", _) => {
                if child_status[i].1 == "loaded" {
                    thee_collector.push((
                        child_status[i].0.to_string(),
                        true,
                        "Service exists and loaded correctly".to_string(),
                    ));
                } else {
                    match child_status[i].1 {
                        "not-found" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Service doesn't exist".to_string(),
                        )),
                        "masked" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Service is blocked/masked".to_string(),
                        )),
                        "error" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Configuration error in unit file"
                                .to_string(),
                        )),
                        _ => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Error Unknown".to_string(),
                        )),
                    }
                }
            }
            ("ActiveState", _) => {
                if child_status[i].1 == "active" {
                    thee_collector.push((
                        child_status[i].0.to_string(),
                        true,
                        "Service is running!".to_string(),
                    ));
                } else {
                    match child_status[i].1 {
                        "failed" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Service failed to start or crashed"
                                .to_string(),
                        )),
                        "inactive" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Service is stopped".to_string(),
                        )),
                        _ => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Error Unknown".to_string(),
                        )),
                    }
                }
            }
            // thi handles what? findout and work on it
            ("Result", _) => {
                if child_status[i].1 == "success" {
                    thee_collector.push((
                        child_status[i].0.to_string(),
                        true,
                        "No error".to_string(),
                    ));
                } else {
                    match child_status[i].1 {
                        "exit-code" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Exited with non-zero code".to_string(),
                        )),
                        "timeout" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Start/Stop timeout exceeded".to_string(),
                        )),
                        "signal" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Killed by signal (SIGTERM/SIGKILL)"
                                .to_string(),
                        )),
                        "core-dump" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Crashed and dumped core".to_string(),
                        )),
                        "watchdog" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Watchdog timeout".to_string(),
                        )),
                        "recources" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Resource limit hit".to_string(),
                        )),
                        "start-limit-hit" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Too many restart attempts".to_string(),
                        )),
                        _ => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Error Unknown".to_string(),
                        )),
                    }
                }
            }
            ("MainPID", _) => {
                if let Ok(code) = child_status[i].1.parse::<u32>() {
                    if code == 0 {
                        thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - No process (either failed or special service type)".to_string()))
                    } else {
                        thee_collector.push((
                            child_status[i].0.to_string(),
                            true,
                            "Process is running".to_string(),
                        ))
                    }
                } else {
                    // Need to hndle the error with better way
                    panic!("Error: Failed to parse Code");
                }
            }
            ("CanStart", _) => {
                if child_status[i].1 == "yes" {
                    thee_collector.push((
                        child_status[i].0.to_string(),
                        true,
                        "Service Can Start".to_string(),
                    ));
                } else {
                    match child_status[i].1 {
                        "no" => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Service Cannot be started".to_string(),
                        )),
                        _ => thee_collector.push((
                            child_status[i].0.to_string(),
                            false,
                            "Failed to start service - Error Unknown".to_string(),
                        )),
                    }
                }
            }
            _ => {
                panic!("No Properties for some reason... go fix your code!")
            }
        }
    }

    // thee_collecor would give tuples in the following order
    // MainPID -> Result -> LoadState -> ActiveState -> CanStart
    // to check, run the following:
    // println!("{:#?}", thee_collector);

    return ChildProperties {
        main_pid: thee_collector[0].clone(),
        result: thee_collector[1].clone(),
        load_state: thee_collector[2].clone(),
        active_state: thee_collector[3].clone(),
        can_start: thee_collector[4].clone(),
    };
}

/*
## **All possible values for Starting service (minimal set):**

### **LoadState** (Does service exist?)
```
✓ "loaded"     → Service exists and loaded correctly
✗ "not-found"  → Service doesn't exist
✗ "masked"     → Service is blocked/masked
✗ "error"      → Configuration error in unit file
```

### **ActiveState** (Is it running?)
```
✓ "active"      → Service is running
✗ "failed"      → Service failed to start or crashed
✗ "inactive"    → Service is stopped
```

### **Result** (Why did it fail?)
```
✓ "success"          → No error
✗ "exit-code"        → Exited with non-zero code
✗ "timeout"          → Start/stop timeout exceeded
✗ "signal"           → Killed by signal (SIGTERM/SIGKILL)
✗ "core-dump"        → Crashed and dumped core
✗ "watchdog"         → Watchdog timeout
✗ "resources"        → Resource limit hit
✗ "start-limit-hit"  → Too many restart attempts
```

### **MainPID** (Is process alive?)
```
✓ > 0  → Process is running
✗ = 0  → No process (either failed or special service type)
```

### **CanStart** (Permission check)
```
✓ "yes" → Can be started
✗ "no"  → Cannot be started (permissions/dependencies)
*/
