use engine::{Action, Domain, Order, execute_order};

/// Public API - the ONLY function external UIs should call
/// Converts intent to Order and passes it to the engine
/// Intent format: (domain, action, arguments): (&str, &str, Option<Vec<String>>)
/// Example: ("service", "start", Some(vec!["nginx".to_string()]))
pub fn process_intent(
    domain: &str,
    action: &str,
    arguments: Option<Vec<String>>,
) -> Result<(), String> {
    let order = intent_to_order(domain, action, arguments)?;
    execute_order(order);
    Ok(())
}

/// Internal: converts intent to Order
fn intent_to_order(
    domain: &str,
    action: &str,
    arguments: Option<Vec<String>>,
) -> Result<Order, String> {
    let domain = parse_domain(domain)?;
    let action = parse_action(&domain, action)?;
    Ok(Order {
        domain,
        action,
        arguments,
    })
}

/// Internal: Parse domain string to Domain enum
fn parse_domain(domain: &str) -> Result<Domain, String> {
    match domain {
        "service" | "services" => Ok(Domain::Services),
        _ => Err(format!("Unknown Module: '{}'.\nAvailable: service", domain)),
    }
}

/// Internal: Parse action based on domain context
fn parse_action(domain: &Domain, action: &str) -> Result<Action, String> {
    match domain {
        Domain::Services => parse_service_action(action),
        // Domain::Network => parse_network_action(action),
        // Domain::User => parse_user_action(action),
    }
}

/// Internal: Service-specific actions
fn parse_service_action(action: &str) -> Result<Action, String> {
    match action {
        "list" | "show" => Ok(Action::List),
        "status" => Ok(Action::Status),
        "run" | "start" => Ok(Action::Start(true)),
        "stop" | "kill" => Ok(Action::Start(false)),
        "mask" | "hide" => Ok(Action::Mask(true)),
        "unmask" => Ok(Action::Mask(false)),
        "enable" | "allow" => Ok(Action::Enable(true)),
        "disable" | "deny" => Ok(Action::Enable(false)),
        "reload" | "restart" => Ok(Action::Reload),
        "reset" => Ok(Action::Reset),
        "help" => Ok(Action::Help),
        _ => Err(format!(
            "Unknown service action: '{}'.\nUse 'service help' for available actions.",
            action
        )),
    }
}

// Future: Network-specific actions
// fn parse_network_action(action: &str) -> Result<Action, String> {
//     match action {
//         "configure" | "config" => Ok(Action::Configure),
//         "list" | "show" => Ok(Action::List),
//         "help" => Ok(Action::Help),
//         _ => Err(format!("Unknown network action: '{}'", action)),
//     }
// }
