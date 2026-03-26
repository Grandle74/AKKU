// api/src/_archive/imperative_backup.rs
// Archive — original imperative process_intent() before the declarative refactor.
// Kept for reference only; not compiled.

pub use engine::PropertyValue;
use engine::{execute_order, Domain, Order};
use std::collections::HashMap;

pub fn process_intent(
    domain: &str,
    action_str: &str,
    arguments: Option<Vec<String>>,
) -> Result<(), String> {
    let order = intent_to_order(domain, action_str, arguments)?;
    execute_order(&order);
    Ok(())
}

fn intent_to_order(
    domain: &str,
    action_str: &str,
    arguments: Option<Vec<String>>,
) -> Result<Order, String> {
    let domain = parse_domain(domain)?;

    let target = if action_str == "list" || action_str == "help" || action_str == "reset" {
        action_str.to_string()
    } else {
        arguments
            .as_ref()
            .and_then(|args| args.first())
            .ok_or("Service name required")?
            .clone()
    };

    let desired_properties = convert_action_to_properties(domain.clone(), action_str)?;
    validate_conflicts(domain.clone(), &desired_properties)?;

    Ok(Order {
        domain,
        target,
        desired_properties,
    })
}

fn parse_domain(domain: &str) -> Result<Domain, String> {
    match domain {
        "service" | "services" => Ok(Domain::Services),
        _ => Err(format!("Unknown module: '{}'. Available: service", domain)),
    }
}

fn convert_action_to_properties(
    domain: Domain,
    action: &str,
) -> Result<HashMap<String, PropertyValue>, String> {
    match domain {
        Domain::Services => {
            let mut props = HashMap::new();
            match action {
                "start" | "run" => {
                    props.insert("running".to_string(), PropertyValue::Bool(true));
                }
                "stop" | "kill" => {
                    props.insert("running".to_string(), PropertyValue::Bool(false));
                }
                "enable" | "allow" => {
                    props.insert("enabled".to_string(), PropertyValue::Bool(true));
                }
                "disable" | "deny" => {
                    props.insert("enabled".to_string(), PropertyValue::Bool(false));
                }
                "mask" | "hide" => {
                    props.insert("masked".to_string(), PropertyValue::Bool(true));
                }
                "unmask" => {
                    props.insert("masked".to_string(), PropertyValue::Bool(false));
                }
                "reload" | "restart" => {
                    props.insert("running".to_string(), PropertyValue::Bool(true));
                }
                "status" | "list" | "help" | "reset" => {} // Meta actions — no properties.
                _ => {
                    return Err(format!(
                        "Unknown service action: '{}'. Use 'service help' for available actions.",
                        action
                    ))
                }
            }
            Ok(props)
        }
    }
}
