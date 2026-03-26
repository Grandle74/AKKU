use engine::{Action, Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{EngineResult, Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

/// Returned by process_tri_intent.
/// When pending_plan is Some, the frontend must ask for approval
/// and call approve_intent() with the user's decision.
/// When None, output is final — no further action needed.
pub use engine::EngineResult as IntentResult;

// ── Public API ───────────────────────────────────────────────────────────────

/// Intent with no target (list, help, reset, ...).
/// Always executes immediately — no planning, no approval.
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = parse_action(action_str).map_err(|e| vec![e])?;

    match action {
        Action::Meta(_) => {
            // Meta actions are fire-and-forget — result is always final.
            let result = execute_order(
                Order {
                    domain,
                    action,
                    target: None,
                    desired_properties: HashMap::new(),
                },
                false,
            )?;
            Ok(result.output) // pending_plan is always None here
        }
        _ => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
    }
}

/// Intent with a target and optional properties (status, start, config, ...).
/// Config actions return a pending plan — caller must handle approval.
/// All other actions return final output with no pending plan.
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<IntentResult, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = parse_action(&action_str).map_err(|e| vec![e])?;

    match action {
        Action::Meta(_) => {
            return Err(vec![format!("Invalid command — see '{} help'", domain_str)]);
        }
        Action::Config => {
            if properties.is_empty() {
                return Err(vec![format!(
                    "✗ No properties provided — see '{} help'",
                    domain_str
                )]);
            }
            // Conflict validation happens at API level — before touching the engine.
            validate_conflicts(domain.clone(), &properties).map_err(|e| vec![e])?;
        }
        Action::Custom(_) => {} // no pre-validation needed, execute directly
    }

    execute_order(
        Order {
            domain,
            action,
            target: Some(target),
            desired_properties: properties,
        },
        false,
    )
}

/// Trip 2 — Send user's approval decision to the engine.
/// Call this only when process_tri_intent returned a pending_plan.
pub fn approve_intent(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    engine_approve(plan, approved)
}

// ── Conflict Validation ───────────────────────────────────────────────────────

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
