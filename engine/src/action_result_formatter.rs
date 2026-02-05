use super::Order;

pub fn action_output(order: Order, action: &str) {
    // Handle the case where arguments might be None or empty
    let arguments = match &order.arguments {
        Some(args) if !args.is_empty() => args,
        _ => {
            println!("✗ Service {} failed → No service name provided", action);
            return;
        }
    };

    let service_name = &arguments[0];

    // Make a general action output formatter
    let result = match action {
        "starting" => services::start_service(&order.arguments),
        "stopping" => services::stop_service(&order.arguments),
        "masking" => services::mask_service(&order.arguments),
        "unmasking" => services::unmask_service(&order.arguments),
        "enabling" => services::enable_service(&order.arguments),
        "disabling" => services::disable_service(&order.arguments),
        //"reloading" => services::reload_service(&order.arguments),
        _ => Err(vec!["Invalid action".to_string()]),
    };

    match result {
        // Everything goes well here!
        Ok(vals) => {
            println!("✓ Service {} succeeded → {}.service", action, service_name);
            for val in &vals {
                println!("   → {}", val);
            }
        }
        // Here we handle Service Error Case
        Err(vals) => {
            println!("✗ Service {} failed → {}.service", action, service_name);
            for val in &vals {
                println!("   → {}", val);
            }
        }
    }
}
