// api/src/lib.rs
pub use engine::PropertyValue;
use engine::{Domain, Order, execute_order};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════
//              ORIGINAL IMPERATIVE FUNCTIONS!
// ═══════════════════════════════════════════════════════════

pub fn process_intent(
    domain: &str,
    action_str: &str,
    arguments: Option<Vec<String>>,
) -> Result<(), String> {
    let order = intent_to_order(domain, action_str, arguments)?;
    execute_order(&order);
    Ok(())
}

// FIX the target extraction logic

fn intent_to_order(
    domain: &str,
    action_str: &str,
    arguments: Option<Vec<String>>,
) -> Result<Order, String> {
    let domain = parse_domain(domain)?;

    // Extract target - FIXED LOGIC
    let target = if action_str == "list" || action_str == "help" || action_str == "reset" {
        // Meta actions use action as target
        action_str.to_string()
    } else {
        // Regular actions (start, stop, status, etc.) need target from arguments
        arguments
            .as_ref()
            .and_then(|args| args.first())
            .ok_or("Service name required")?
            .clone()
    };

    // Convert action to properties
    let desired_properties = convert_action_to_properties(domain.clone(), action_str)?;

    // Validate conflicts BEFORE creating order
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
        // "net" | "network" => Ok(Domain::Networks), <- for future Network Module
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
                "status" | "list" | "help" | "reset" => {
                    // Meta actions - no properties
                }
                _ => {
                    return Err(format!(
                        "Unknown service action: '{}'. Use 'service help' for available actions.",
                        action
                    ));
                }
            }

            Ok(props)
        }
    }
}
