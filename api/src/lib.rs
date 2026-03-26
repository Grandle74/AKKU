// api/src/lib.rs
use engine::{Action, Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{EngineResult, Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

// IntentResult is the API-level alias for EngineResult.
// When `pending_plan` is Some, the frontend must ask for approval and call `approve_intent()`.
// When None, `output` is final — no further action needed.
pub use engine::EngineResult as IntentResult;

// ── Public API ───────────────────────────────────────────────────────────────

/// Bi-intent: domain + action, no target (list, help, reset, ...).
/// Always executes immediately — no planning, no approval.
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = Action::from(action_str);

    match action {
        Action::Meta(_) => {
            let result = execute_order(
                Order {
                    domain,
                    action,
                    target: None,
                    desired_properties: HashMap::new(),
                },
                false,
            )?;
            Ok(result.output)
        }
        _ => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
    }
}

/// Tri-intent: domain + action + target + optional properties (status, start, config, ...).
/// Config actions return a pending Plan — the caller must handle approval.
/// All other actions return final output with no pending plan.
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
) -> Result<IntentResult, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = Action::from(action_str.as_str());

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
            // Conflict validation happens at the API level — before touching the engine.
            validate_conflicts(domain.clone(), &properties).map_err(|e| vec![e])?;
        }
        Action::Custom(_) => {} // No pre-validation needed; execute directly.
    }

    execute_order(
        Order {
            domain,
            action,
            target: Some(target),
            desired_properties: properties,
        },
        true,
    )
}

/// Trip 2 — Forward the user's approval decision to the engine.
/// Call this only when `process_tri_intent` returned a `pending_plan`.
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
