// api/src/lib.rs
pub use engine::PropertyValue;
use engine::{Action, Domain, EngineConfig, Order, execute_order};
use std::collections::HashMap;
mod service_validator;

/// Intent with no target (list, help, reset, ...)
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = parse_action(action_str).map_err(|e| vec![e])?;

    match action {
        Action::Meta(_) =>
        // Execution Result of execute_order is ignored temporarily
        {
            execute_order(
                Order {
                    domain,
                    action,
                    target: None,
                    desired_properties: HashMap::new(),
                },
                EngineConfig {
                    dry_run: true,
                    auto_approve: false,
                },
            )
        }

        _ => Err(vec![format!(
            "✗ Invalid command — see '{} help'",
            domain_str
        )]),
    }
}

/// Intent with a target, and optionally desired properties (status, start, change, ...)
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = parse_action(&action_str).map_err(|e| vec![e])?;

    match action {
        Action::Meta(_) => {
            return Err(vec![format!(
                "✗ Invalid command — see '{} help'",
                domain_str
            )]);
        }
        Action::Config => {
            if properties.is_empty() {
                return Err(vec![format!(
                    "✗ No properties provided — see '{} help'",
                    domain_str
                )]);
            }
            validate_conflicts(domain.clone(), &properties).map_err(|e| vec![e])?;
        }
        Action::Custom(_) => {} // fine, just execute
    }

    execute_order(
        Order {
            domain,
            action,
            target: Some(target),
            desired_properties: properties,
        },
        EngineConfig {
            dry_run: true,
            auto_approve: true,
        },
    )
}
// ── Conflict validation — dispatches per domain ──────────────────────────────
// Alhamdulillah ── it's located in the actual Module Crate
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
        "service" | "services" | "srv" => Ok(Domain::Services),
        _ => Err(format!("Unknown module '{}' — available: service", s)),
    }
}

fn parse_action(s: &str) -> Result<Action, String> {
    match s {
        "list" => Ok(Action::Meta("list".to_string())),
        "help" => Ok(Action::Meta("help".to_string())),
        "reset" => Ok(Action::Meta("reset".to_string())),
        "config" | "change" | "cfg" => Ok(Action::Config),
        _ => Ok(Action::Custom(s.to_string())),
    }
}
