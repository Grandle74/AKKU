// api/src/lib.rs
//
// The API layer: the single entry point from the CLI into YaST3.
//
// Responsibilities:
//   1. Parse and validate the request (domain, action, properties) — reject early.
//   2. Call the engine with a well-formed Order.
//   3. Resolve the engine's result into an IntentOutcome based on the run mode.
//   4. Forward approval decisions back to the engine (Trip 2).
//
// The API does NOT execute systemctl commands and does NOT build Steps.
// It also does NOT render output — that is the CLI's job.

use engine::{Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

pub use engine::EngineResult as IntentResult;

// Re-exported so the CLI never imports shared_libs or engine directly.
// The CLI's only dependency is this crate.
pub use engine::Action;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the CLI to render.
pub enum IntentOutcome {
    /// Action completed immediately — display the output lines.
    Immediate(Vec<String>),
    /// Dry-run: plan was generated but nothing was saved or executed.
    DryRun { plan_text: Vec<String> },
    /// Normal flow: plan saved, awaiting explicit user approval.
    RequiresApproval { plan: Plan, plan_text: Vec<String> },
    /// Force flow: plan was auto-approved and executed successfully.
    AutoApplied {
        plan_text: Vec<String>,
        result_text: Vec<String>,
    },
    /// Force flow: plan was auto-approved but execution failed.
    ApplyFailed {
        plan_text: Vec<String>,
        errors: Vec<String>,
    },
}

/// Controls how a Config intent is handled after planning.
pub enum RunMode {
    Normal, // Prompt user for approval.
    DryRun, // Show the plan, execute nothing.
    Force,  // Auto-approve without prompting.
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Trip 1, bi-intent path: handles Meta actions (list, help, reset) that need
/// no target and no properties. Rejects anything other than a Meta action.
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = Action::from(action_str);

    match action {
        Action::Meta(_) => {
            let result = execute_order(Order {
                domain,
                action,
                target: None,
                desired_properties: HashMap::new(),
            })?;
            Ok(result.output)
        }
        _ => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
    }
}

/// Trip 1, tri-intent path: handles all actions that have a target, including
/// imperative Custom actions and declarative Config actions with properties.
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
    mode: &RunMode,
) -> Result<IntentOutcome, Vec<String>> {
    let action = Action::from(action_str.as_str());
    let domain = validate_request(domain_str, &action, &properties)?;

    let result = execute_order(Order {
        domain,
        action: action.clone(),
        target: Some(target),
        desired_properties: properties,
    })?;

    // Non-Config actions always execute immediately — the engine returns no plan.
    if !matches!(action, Action::Config) {
        return Ok(IntentOutcome::Immediate(result.output));
    }

    // Config actions: the engine either found nothing to do (None) or
    // returned a plan for us to handle according to the run mode.
    resolve_outcome(result, mode)
}

/// Trip 2: forwards a user's approval decision to the engine.
pub fn approve_intent(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    engine_approve(plan, approved)
}

// ── Internal Helpers ──────────────────────────────────────────────────────────

/// Validates the request at the API boundary: known domain, legal action for
/// this context, and (for Config) conflict-free properties.
fn validate_request(
    domain_str: &str,
    action: &Action,
    properties: &HashMap<String, PropertyValue>,
) -> Result<Domain, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;

    match action {
        // Meta actions belong in bi-intent — they should never reach here.
        Action::Meta(_) => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
        // Config requires at least one property; empty is a user mistake.
        Action::Config if properties.is_empty() => Err(vec![format!(
            "No properties provided — see '{} help'",
            domain_str
        )]),
        // Config with properties: run domain-specific conflict validation.
        Action::Config => {
            validate_conflicts(domain.clone(), properties).map_err(|e| vec![e])?;
            Ok(domain)
        }
        // Custom actions pass through — the module handles unknown action names.
        Action::Custom(_) => Ok(domain),
    }
}

/// Routes a completed engine result to the appropriate IntentOutcome based on run mode.
///
/// Called only for Config actions. By the time we reach here, the engine has
/// already confirmed there is work to do (plan is Some) or not (plan is None).
fn resolve_outcome(result: IntentResult, mode: &RunMode) -> Result<IntentOutcome, Vec<String>> {
    let plan_text = result.output;

    // Engine returned None: the service is already at desired state.
    let Some(plan) = result.pending_plan else {
        return Ok(IntentOutcome::Immediate(plan_text));
    };

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan_text }),

        RunMode::Force => {
            // Save the plan file for audit record before executing.
            save_plan(&plan)?;
            match approve_intent(plan, true) {
                Ok(result_text) => Ok(IntentOutcome::AutoApplied {
                    plan_text,
                    result_text,
                }),
                Err(errors) => Ok(IntentOutcome::ApplyFailed { plan_text, errors }),
            }
        }

        RunMode::Normal => {
            // Save plan file before handing back to CLI — guarantees an audit record
            // exists even if the user's answer is interrupted.
            save_plan(&plan)?;
            Ok(IntentOutcome::RequiresApproval { plan, plan_text })
        }
    }
}

/// Serializes and persists the plan via the engine's public surface.
///
/// The API never touches plan_store directly — that is a private engine
/// implementation detail. This helper exists to avoid repeating the
/// serialize + map_err pattern for both Force and Normal paths.
fn save_plan(plan: &Plan) -> Result<(), Vec<String>> {
    let json = serde_json::to_string_pretty(plan).map_err(|e| vec![e.to_string()])?;
    engine::save_plan(&json, &plan.id).map_err(|e| vec![e])
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
