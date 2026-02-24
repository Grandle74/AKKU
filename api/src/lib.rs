// api/src/lib.rs
pub use engine::PropertyValue;
use engine::{Domain, Order, execute_order};
use std::collections::HashMap;
mod service_validator;

// In case of having a non target command, Frontend calls this function
pub fn process_bi_command(domain_str: &str, action_str: &str) -> Result<(), String> {
    let domain = parse_domain(domain_str)?;

    // Extract target - FIXED LOGIC
    //if action_str == "list" || action_str == "help" || action_str == "reset" {
    let order = Order {
        domain,
        target: action_str.to_string(),
        desired_properties: HashMap::new(),
    };
    execute_order(&order);
    Ok(())
    // } else {
    //     Err(format!("✗ Invalid Command: See '{} help'", domain_str).to_string())
    // }
}

// ═══════════════════════════════════════════════════════════
//      NEW: REPLACE IMPERATIVE WITH DECLARATIVE FUNCTION
// ═══════════════════════════════════════════════════════════

// This function is the responsible of generating the Order using the Intent Data given by the Frontend
// It Parses the Domain String into Domain Type/Enum
pub fn process_tri_command(
    domain: &str,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<(), String> {
    let domain = parse_domain(domain)?;

    // Validate CONFLICTS before generating the order
    validate_conflicts(domain.clone(), &properties)?;

    let order = Order {
        domain,
        target,
        desired_properties: properties,
    };

    execute_order(&order);
    Ok(())
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

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    match props.get(key) {
        Some(PropertyValue::Bool(b)) => Some(*b),
        _ => None,
    }
}
