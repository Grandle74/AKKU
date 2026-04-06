use engine::{Action, Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

pub use engine::EngineResult as IntentResult;

pub enum IntentOutcome {
    Immediate(Vec<String>),
    DryRun {
        plan_text: Vec<String>,
    },
    RequiresApproval {
        plan: Plan,
        plan_text: Vec<String>,
    },
    AutoApplied {
        plan_text: Vec<String>,
        result_text: Vec<String>,
    },
}

pub enum RunMode {
    Normal,
    DryRun,
    Force,
}

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

pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
    mode: &RunMode,
) -> Result<IntentOutcome, Vec<String>> {
    let action = Action::from(action_str.as_str());
    let domain = validate_request(domain_str, &action, &properties)?;

    let result = execute_order(
        Order {
            domain,
            action: action.clone(), // Clone so we can check it later
            target: Some(target),
            desired_properties: properties,
        },
        true,
    )?;

    // FIX: If it wasn't a Config action, it's an Immediate result.
    // The Engine correctly returned pending_plan: None for these.
    if !matches!(action, Action::Config) {
        return Ok(IntentOutcome::Immediate(result.output));
    }

    // Only Config actions go to the resolver to handle DryRun/Force/Normal
    resolve_outcome(result, mode)
}

pub fn approve_intent(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    engine_approve(plan, approved)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn validate_request(
    domain_str: &str,
    action: &Action,
    properties: &HashMap<String, PropertyValue>,
) -> Result<Domain, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;

    match action {
        Action::Meta(_) => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
        Action::Config if properties.is_empty() => Err(vec![format!(
            "No properties provided — see '{} help'",
            domain_str
        )]),
        Action::Config => {
            validate_conflicts(domain.clone(), properties).map_err(|e| vec![e])?;
            Ok(domain)
        }
        _ => Ok(domain),
    }
}

fn resolve_outcome(result: IntentResult, mode: &RunMode) -> Result<IntentOutcome, Vec<String>> {
    let plan_text = result.output;
    let plan = result
        .pending_plan
        .ok_or_else(|| vec!["Engine failed to generate plan".into()])?;

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan_text }),
        RunMode::Force => {
            let result_text = approve_intent(plan, true)?;
            Ok(IntentOutcome::AutoApplied {
                plan_text,
                result_text,
            })
        }
        RunMode::Normal => Ok(IntentOutcome::RequiresApproval { plan, plan_text }),
    }
}

fn validate_conflicts(
    domain: Domain,
    properties: &HashMap<String, PropertyValue>,
) -> Result<(), String> {
    match domain {
        Domain::Services => service_validator::validate(properties),
    }
}

fn parse_domain(s: &str) -> Result<Domain, String> {
    match s {
        "service" | "services" | "srv" => Ok(Domain::Services),
        _ => Err(format!("Unknown module '{}' — available: services", s)),
    }
}
