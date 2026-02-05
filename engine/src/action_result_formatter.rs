use super::Order;

// Action Output should be turned to ActionOutputFormatter
// This is a temporary solution until we have a proper formatter
// Proper formatter will be needed when Engine gets ready

pub fn action_output(order: Order, action: &str) {
    // Handle special case: reset doesn't need service name
    if action == "resetting" {
        let result = services::reset_service();
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

    // Handle the case where arguments might be None or empty
    let arguments = match &order.arguments {
        Some(args) if !args.is_empty() => args,
        _ => {
            println!("✗ Service {} failed → No service name provided", action);
            return;
        }
    };

    let service_name = &arguments[0];

    // Execute the appropriate action
    let result = match action {
        "starting" => services::start_service(&order.arguments),
        "stopping" => services::stop_service(&order.arguments),
        "masking" => services::mask_service(&order.arguments),
        "unmasking" => services::unmask_service(&order.arguments),
        "enabling" => services::enable_service(&order.arguments),
        "disabling" => services::disable_service(&order.arguments),
        "reloading" => services::reload_service(&order.arguments),
        _ => {
            println!("✗ Unknown action: {}", action);
            return;
        }
    };

    // Format output based on result
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
