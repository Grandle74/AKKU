use crate::{Domain, Order};
use std::time::Instant;

pub fn action_output(order: Order, action: &str) {
    let start_time = Instant::now();

    // Create log entry
    let domain = match order.domain {
        Domain::Services => "service",
        // Domain::Network => "network",  // future
        // Domain::User => "user",        // future
    };
    let target = match &order.arguments {
        Some(args) if !args.is_empty() => &args[0],
        _ => "unknown",
    };

    let mut log = logger::OperationLog::new(domain, action, target);

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

    // Handle normal actions
    let service_name = match &order.arguments {
        Some(args) if !args.is_empty() => &args[0],
        _ => {
            println!("✗ Service {} failed → No service name provided", action);
            let result: Result<Vec<String>, Vec<String>> =
                Err(vec!["No service name provided".to_string()]);
            let duration = start_time.elapsed().as_millis() as u64;
            log = log.finish(result, duration);
            let _ = log.save();
            return;
        }
    };

    // Execute action
    let result = match action {
        "starting" => services::start_service(&order.arguments),
        "stopping" => services::stop_service(&order.arguments),
        "masking" => services::mask_service(&order.arguments),
        "unmasking" => services::unmask_service(&order.arguments),
        "enabling" => services::enable_service(&order.arguments),
        "disabling" => services::disable_service(&order.arguments),
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
            println!("✓ Service {} succeeded → {}.service", action, service_name);
            for val in &vals {
                println!("   → {}", val);
            }
        }
        Err(vals) => {
            println!("✗ Service {} failed → {}.service", action, service_name);
            for val in &vals {
                println!("   → {}", val);
            }
        }
    }
}
