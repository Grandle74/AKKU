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

    // temporary functionality - will be generalized in shaa' allah
    let result = if action == "starting" {
        services::start_service(&order.arguments)
    } else {
        services::stop_service(&order.arguments)
    };

    match result {
        // Everything goes well here! XD
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
