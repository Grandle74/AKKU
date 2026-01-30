use std::process::Command;

#[derive(Debug)]
pub struct ChildProperties {
    // Property: (Value, Fail/Success, Reason)
    pub load_state: String,
    pub active_state: String,
    pub result: String,
    pub main_pid: String,
    pub can_start: String,
}

impl ChildProperties {
    pub fn new(service: String) -> Self {
        let mut prop = Self {
            load_state: "".to_string(),
            active_state: "".to_string(),
            result: "".to_string(),
            main_pid: "".to_string(),
            can_start: "".to_string(),
        };

        prop.prop_parser(service);
        return prop;
    }

    fn prop_parser(&mut self, service: String) {
        // This adds a small delay to let systemd update the status -- it's the only solution
        std::thread::sleep(std::time::Duration::from_millis(3000));
        // "child_status" is the needed status of the child command to catch error efficiently
        let child_status = Command::new("systemctl")
            .args([
                "show",
                &service,
                //the following are all the needed properties to cover 100% of results
                "--property=LoadState,CanStart,Result,ActiveState,MainPID",
            ])
            .output()
            .expect("Failed to check status");

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
        // dbg
        // println!("{:?}", child_status);

        // Transforming "child_status" into a Valid Struct "ChildProperties"
        // i.e.: Storing each Property Value with its right Struct Field
        for i in 0..child_status.len() {
            match child_status[i].0 {
                "LoadState" => {
                    self.load_state = child_status[i].1.to_string();
                }
                "ActiveState" => {
                    self.active_state = child_status[i].1.to_string();
                }
                "Result" => {
                    self.result = child_status[i].1.to_string();
                }
                "MainPID" => {
                    self.main_pid = child_status[i].1.to_string();
                }
                "CanStart" => {
                    self.can_start = child_status[i].1.to_string();
                }
                _ => {
                    panic!("No Properties for some reason... go fix your code!")
                }
            }
        }
    }
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
