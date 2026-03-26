// engine/src/action_result_formatter.rs
use crate::Order;
use std::time::Instant;

pub fn action_output(order: &Order, action: &str) {
    let start_time = Instant::now();

    let domain = match order.domain {
        crate::Domain::Services => "service",
    };

    let mut log = logger::OperationLog::new(domain, action, &order.target);

    // Handle special case: reset
    if action == "resetting" {
        let result = services::reset_service();
        let duration = start_time.elapsed().as_millis() as u64;
        log = log.finish(result.clone(), duration);
        let _ = log.save();

        match result {
            Ok(vals) => {
                println!("✓ Service {} succeeded", action);
                for val in &vals {
                    println!("   → {}", val);
                }
            }
            Err(vals) => {
                println!("✗ Service {} failed", action);
                for val in &vals {
                    println!("   → {}", val);
                }
            }
        }
        return;
    }

    // Execute action
    let args = Some(vec![order.target.clone()]);
    let result = match action {
        "starting" => services::start_service(&args),
        "stopping" => services::stop_service(&args),
        "masking" => services::mask_service(&args),
        "unmasking" => services::unmask_service(&args),
        "enabling" => services::enable_service(&args),
        "disabling" => services::disable_service(&args),
        "reloading" => services::reload_service(&args),
        _ => {
            println!("✗ Unknown action: {}", action);
            return;
        }
    };

    let duration = start_time.elapsed().as_millis() as u64;
    log = log.finish(result.clone(), duration);
    let _ = log.save();

    // Format output
    match result {
        Ok(vals) => {
            println!("✓ Service {} succeeded → {}.service", action, order.target);
            for val in &vals {
                println!("   → {}", val);
            }
        }
        Err(vals) => {
            println!("✗ Service {} failed → {}.service", action, order.target);
            for val in &vals {
                println!("   → {}", val);
            }
        }
    }
}
