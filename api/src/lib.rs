// api/src/lib.rs
pub use engine::PropertyValue;
use engine::{Action, Domain, Order, execute_order};
use std::collections::HashMap;
mod service_validator;

// In case of having a non target command, Frontend calls this function
pub fn process_bi_command(domain_str: &str, action_str: &String) -> Result<(), String> {
    let domain = parse_domain(domain_str)?;
    let action = parse_action(action_str)?;

    match action {
        Action::List | Action::Help | Action::Reset => {
            let order = Order {
                domain,
                action: action,
                target: "".to_string(),
                desired_properties: HashMap::new(),
            };
            execute_order(&order);
            Ok(())
        }
        _ => Err(format!("✗ Invalid Command: See '{} help'", domain_str).to_string()),
    }
}

// ═══════════════════════════════════════════════════════════
//      NEW: REPLACE IMPERATIVE WITH DECLARATIVE FUNCTION
// ═══════════════════════════════════════════════════════════

// This function is the responsible of generating the Order using the Intent Data given by the Frontend
// It Parses the Domain String into Domain Type/Enum
pub fn process_tri_command(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<(), String> {
    let domain = parse_domain(domain_str)?;
    let action = parse_action(&action_str)?;

    match action {
        Action::Status | Action::Reload => {
            let order = Order {
                domain,
                action,
                target,
                desired_properties: properties,
            };

            execute_order(&order);
            Ok(())
        }
        Action::Config => {
            validate_conflicts(domain.clone(), &properties)?;
            let order = Order {
                domain,
                action,
                target,
                desired_properties: properties,
            };

            execute_order(&order);
            Ok(())
        }
        _ => Err(format!("✗ Invalid Command: See '{} help'", domain_str).to_string()),
    }

    // Validate CONFLICTS before generating the order
}

// ════════════════════════════════════════════════════════════
//  NEW: CONFLICT VALIDATION - Generalized for future modules
// ════════════════════════════════════════════════════════════

fn validate_conflicts(
    domain: Domain,
    properties: &HashMap<String, PropertyValue>,
) -> Result<(), String> {
    match domain {
        Domain::Services => service_validator::validate(properties),
        // Future: Domain::Network => validate_network_conflicts(properties),
    }
}

fn parse_domain(domain: &str) -> Result<Domain, String> {
    match domain {
        "service" | "services" => Ok(Domain::Services),
        // "net" | "network" => Ok(Domain::Networks), <- for future Network Module
        _ => Err(format!("Unknown module: '{}'\nAvailable: service", domain)),
    }
}

fn parse_action(action: &String) -> Result<Action, String> {
    match action.as_str() {
        "start" => Ok(Action::Start),
        "stop" => Ok(Action::Stop),
        "enable" => Ok(Action::Enable),
        "disable" => Ok(Action::Disable),
        "mask" => Ok(Action::Mask),
        "unmask" => Ok(Action::Unmask),
        "reload" => Ok(Action::Reload),
        "status" => Ok(Action::Status),
        "list" => Ok(Action::List),
        "help" => Ok(Action::Help),
        "reset" => Ok(Action::Reset),
        "change" | "config" => Ok(Action::Config),
        _ => Err("✗ Error: Invalid command action".to_string()),
    }
}

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    match props.get(key) {
        Some(PropertyValue::Bool(b)) => Some(*b),
        _ => None,
    }
}
