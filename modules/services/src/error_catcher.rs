use std::process::Command;

#[derive(Debug)]
pub struct ChildProperties {
    pub load_state: String,            // preflight only
    pub active_state: String,          // required
    pub sub_state: String,             // required
    pub result: String,                // required
    pub exec_main_status: Option<i32>, // numeric truth
    pub main_pid: Option<u32>,         // informational
}

impl ChildProperties {
    pub fn new(service: String) -> Self {
        let mut prop = Self {
            load_state: "".to_string(),
            active_state: "".to_string(),
            sub_state: "".to_string(),
            result: "".to_string(),
            main_pid: None,
            exec_main_status: None,
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
                "--property=LoadState,ExecMainStatus,Result,ActiveState,MainPID,SubState",
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
                    self.main_pid = {
                        if child_status[i].1.to_string().parse::<u32>().is_ok() {
                            Some(child_status[i].1.to_string().parse::<u32>().unwrap())
                        } else {
                            None
                        }
                    };
                }
                "ExecMainStatus" => {
                    self.exec_main_status = {
                        if child_status[i].1.to_string().parse::<i32>().is_ok() {
                            Some(child_status[i].1.to_string().parse::<i32>().unwrap())
                        } else {
                            None
                        }
                    };
                }
                "SubState" => {
                    self.sub_state = child_status[i].1.to_string();
                }
                _ => {
                    panic!("No Properties for some reason... go fix your code!")
                }
            }
        }
    }
}

pub fn start_validation(service: Vec<String>) {
    let props = ChildProperties::new(service[0].clone());
    // dbg
    // println!("{:#?}", props);
    let mut vals: Vec<String> = Vec::new();
    let mut error_counter: u8 = 0;

    // 1st Layer
    match props.load_state.as_str() {
        "loaded" => {
            vals.push("Service exists and loaded correctly".to_string());
        }
        _ => {
            vals.push("Service doesn't exist".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
    }

    // Not a Layer
    // Aslong as the service is loaded, the main PID is not None
    if props.main_pid.is_some() {
        vals.push(format!("Main PID: {}", props.main_pid.unwrap()));
    } else {
        vals.push(format!(
            "Main PID: {} - not running",
            props.main_pid.unwrap()
        ));
    }

    // 2nd Layer
    match props.exec_main_status {
        Some(0) => {}
        Some(126) => {
            vals.push("Permission denied".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        Some(127) => {
            vals.push("Executable not found".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        Some(status) if status >= 128 => {
            vals.push(format!("Service crashed via signal {}", status - 128));
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        _ => {
            if let Some(petipain) = props.exec_main_status {
                vals.push(format!("Exec Main Status: {}", petipain));
                error_counter += 1;
                output(vals, service, error_counter);
                return;
            }
        }
    }

    // 3rd Layer
    if props.result.as_str() != "success" {
        error_counter += 1;
        output(vals, service, error_counter);
        return;
    }

    // 4th Layer
    match props.active_state.as_str() {
        "active" => {
            vals.push("Service is running".to_string());
        }
        "activating" | "deactivating" => {
            vals.push("Action stuck - Timeout".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        _ => {
            vals.push("Failed to start service".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
    }

    // 5th Layer
    match props.sub_state.as_str() {
        "running" => {
            //println!("Service is running")
        }
        "exited" => {
            vals.push("Service exited with error".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        "failed" => {
            vals.push("Service failed to start".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        "auto-restart" => {
            vals.push("Service crashed".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        "dead" => {
            vals.push("Service died unexpectedly".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
        _ => {
            vals.push("Service status unknown".to_string());
            error_counter += 1;
            output(vals, service, error_counter);
            return;
        }
    }
    output(vals, service, error_counter);
}

fn output(vals: Vec<String>, service: Vec<String>, error_counter: u8) {
    // Output Design

    if error_counter > 0 {
        println!("✗ Service starting failed → {}.service", service[0]);
    } else {
        println!("✓ Service starting successed → {}.service", service[0]);
    }

    for s in 0..vals.len() {
        println!("   → {}", vals[s]);
    }
}
