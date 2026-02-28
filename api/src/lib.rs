// api/src/lib.rs
pub use engine::PropertyValue;
use engine::{Action, Domain, Order, execute_order};
use std::collections::HashMap;
mod service_validator;

/// Intent with no target (list, help, reset, ...)
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<(), String> {
    let domain = parse_domain(domain_str)?;
    let action = parse_action(action_str)?;

    match action {
        Action::List | Action::Help | Action::Reset => {
            execute_order(&Order {
                domain,
                action,
                target: String::new(),
                desired_properties: HashMap::new(),
            });
            Ok(())
        }
        _ => Err(format!("✗ Invalid command — see '{} help'", domain_str)),
    }
}

/// Intent with a target, and optionally desired properties (status, start, change, ...)
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<(), String> {
    let domain = parse_domain(domain_str)?;
    let action = parse_action(&action_str)?;

    match action {
        // Read-only actions — just need the target
        Action::Status | Action::Reload => {
            execute_order(&Order {
                domain,
                action,
                target,
                desired_properties: properties,
            });
            Ok(())
        }

        // Declarative — needs target + at least one property
        Action::Config => {
            if properties.is_empty() {
                return Err(format!(
                    "✗ No properties provided — see '{} help'",
                    domain_str
                ));
            }
            validate_conflicts(domain.clone(), &properties)?;
            execute_order(&Order {
                domain,
                action,
                target,
                desired_properties: properties,
            });
            Ok(())
        }

        _ => Err(format!("✗ Invalid command — see '{} help'", domain_str)),
    }
}

// ── Conflict validation — dispatches per domain ──────────────────────────────

fn validate_conflicts(
    domain: Domain,
    properties: &HashMap<String, PropertyValue>,
) -> Result<(), String> {
    match domain {
        Domain::Services => service_validator::validate(properties),
        // Future: Domain::Network => network_validator::validate(properties),
    }
}

// ── Parsers ───────────────────────────────────────────────────────────────────

fn parse_domain(s: &str) -> Result<Domain, String> {
    match s {
        "service" | "services" => Ok(Domain::Services),
        _ => Err(format!("Unknown module '{}' — available: service", s)),
    }
}

fn parse_action(s: &str) -> Result<Action, String> {
    match s {
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
        _ => Err(format!("Unknown action '{}'", s)),
    }
}
