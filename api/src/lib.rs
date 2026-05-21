// api/src/lib.rs
//
// The single entry point from the frontend into AKKU.
//
// Does NOT execute init system commands, build Steps, or persist plans —
// those belong to the engine. Does NOT render output — that belongs to
// the frontend.
//
// Rollback architecture:
//   Auto-rollback  — triggered here when approve_intent detects execution
//                    failure on the Normal path. --force fails and leaves
//                    state as-is; the snapshot is on disk for manual rollback.
//   Manual rollback — triggered by rollback_intent(), called by the frontend
//                     after the user picks a plan from the plan history view.

pub use engine::{Action, PropertyValue};
use engine::{Domain, EngineResult, Order, approve_plan as engine_approve, execute_order};
use std::collections::HashMap;

mod service_validator;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the frontend to render.
pub enum IntentOutcome {
    /// Action completed immediately — returns the output lines.
    Immediate(Vec<String>),

    /// Dry-run: returns the plan text but nothing was saved or executed.
    DryRun { plan_text: Vec<String> },

    /// Normal Path (Trip 1): plan saved, awaiting explicit user approval.
    RequiresApproval { plan_id: String },

    /// Forced path only: plan was executed immediately successfully.
    Applied {
        plan_id: String,
        result_text: Vec<String>,
    },

    /// Forced path only: plan was executed immediately and failed.
    ApplyFailed {
        plan_id: String,
        exec_errors: Vec<String>,
    },

    /// User-approved execution (Trip 2) failed and auto-rollback restored previous state.
    ///
    /// `plan_text` is absent — the frontend already rendered the plan before the
    /// approval prompt. `rollback_text` is raw executor output; the frontend owns
    /// the rollback header.
    ApplyFailedRolledBack {
        exec_errors: Vec<String>,
        rollback_text: Vec<String>,
    },

    /// User-approved execution (Trip 2) failed and auto-rollback also failed.
    ///
    /// `plan_text` is absent for the same reason as `ApplyFailedRolledBack`.
    ApplyFailedRollbackFailed {
        exec_errors: Vec<String>,
        rollback_errors: Vec<String>,
    },

    /// Manual rollback completed successfully.
    ///
    /// `rollback_text` is raw executor output; the frontend owns the rollback plan header.
    RolledBack {
        origin_plan_id: String,
        rollback_text: Vec<String>,
    },

    /// Manual rollback failed.
    RollbackFailed {
        origin_plan_id: String,
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

/// Trip 1, bi-intent path: execute a Meta action.
///
/// Rejects any action other than Meta — those must go through `process_tri_intent`.
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
                mode: None,
            })?;
            Ok(result.output)
        }
        _ => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
    }
}

/// Trip 1, tri-intent path: execute an action that has a target.
///
/// Handles both imperative Custom actions and declarative Config actions with
/// properties. Config actions with no properties are rejected here.
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
    mode: &RunMode,
) -> Result<IntentOutcome, Vec<String>> {
    let action = Action::from(action_str.as_str());
    let domain = validate_request(domain_str, &action, &properties)?;

    let is_config = matches!(action, Action::Config);
    let result = execute_order(Order {
        domain,
        action,
        target: Some(target),
        desired_properties: properties,
        mode: match (is_config, mode) {
            (true, RunMode::Normal) => Some("normal".to_string()),
            (true, RunMode::Force) => Some("force".to_string()),
            _ => None, // DryRun or non-Config: don't save
        },
    })?;

    if !is_config {
        return Ok(IntentOutcome::Immediate(result.output));
    }

    resolve_outcome(result, mode)
}

/// Trip 2: forward a user's approval decision to the engine.
///
/// On execution failure the API — not the engine — triggers auto-rollback.
/// This is the Normal path's safety net only. --force has no auto-rollback
/// by design: the user asserted control, the snapshot is on disk, and the
/// plan history view allows manual undo.
///
/// Returns `IntentOutcome::Immediate` on both success and rejection (rejection
/// is not an error — the user chose it).
pub fn approve_intent(id: &str, approved: bool) -> IntentOutcome {
    if !approved {
        let _ = engine_approve(id, false);
        return IntentOutcome::Immediate(vec!["Plan rejected.".to_string()]);
    }
    match engine_approve(id, true) {
        Ok(output) => IntentOutcome::Immediate(output),
        Err(exec_errors) => build_rollback_outcome(id, exec_errors),
    }
}

/// Generate and save the rollback plan for a given origin plan without executing it.
///
/// Called by the frontend's plan history view before asking the user to confirm.
/// The returned plan ID is passed to `approve_intent` on confirmation, giving the
/// user a chance to review exactly what will be restored.
pub fn preview_rollback_intent(origin_plan_id: &str) -> Result<(String, Vec<String>), Vec<String>> {
    engine::build_rollback_plan(origin_plan_id)
}

/// Expose a saved plan's text for display by the frontend.
pub fn read_plan(id: &str) -> Result<Vec<String>, String> {
    engine::read_plan(id)
}

// ── Internal Helpers ──────────────────────────────────────────────────────────

/// Validate the request at the API boundary: known domain, legal action for
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
        Action::Config if properties.is_empty() => Err(vec![format!(
            "No properties provided — see '{} help'",
            domain_str
        )]),
        Action::Config => {
            validate_properties_conflicts(domain.clone(), properties).map_err(|e| vec![e])?;
            Ok(domain)
        }
        // Custom actions pass through — the Engine's executor module handles unknown action names.
        Action::Custom(_) => Ok(domain),
    }
}

/// Route a completed engine result to the appropriate `IntentOutcome`.
///
/// Called only for Config actions. `result.pending_plan` being `None` means
/// the target is already at the desired state — nothing to do.
fn resolve_outcome(result: EngineResult, mode: &RunMode) -> Result<IntentOutcome, Vec<String>> {
    let plan_text = result.output;

    let Some(plan_id) = result.pending_plan else {
        return Ok(IntentOutcome::Immediate(plan_text));
    };

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan_text }),

        RunMode::Force => match engine_approve(&plan_id, true) {
            Ok(result_text) => Ok(IntentOutcome::Applied {
                plan_id,
                result_text,
            }),
            // No auto-rollback on --force. Snapshot is on disk; the plan
            // history view lets the user undo manually.
            Err(exec_errors) => Ok(IntentOutcome::ApplyFailed {
                plan_id,
                exec_errors,
            }),
        },

        RunMode::Normal => Ok(IntentOutcome::RequiresApproval { plan_id }),
    }
}

/// Attempt auto-rollback after a user-approved execution failure and return
/// the appropriate structured outcome.
///
/// `plan_text` is intentionally not threaded through — the frontend already
/// rendered the plan before the approval prompt, so including it here would
/// cause a double-print.
fn build_rollback_outcome(plan_id: &str, exec_errors: Vec<String>) -> IntentOutcome {
    let rollback_plan_id = match engine::build_rollback_plan(plan_id) {
        Err(rollback_errors) => {
            return IntentOutcome::ApplyFailedRollbackFailed {
                exec_errors,
                rollback_errors,
            };
        }
        // Empty id means the snapshot delta was zero — nothing to restore.
        Ok((id, _)) if id.is_empty() => {
            return IntentOutcome::ApplyFailedRolledBack {
                exec_errors,
                rollback_text: vec![
                    "Nothing to restore — pre-change state matches current state.".to_string(),
                ],
            };
        }
        Ok((id, _)) => id,
    };

    match engine_approve(&rollback_plan_id, true) {
        Ok(rollback_text) => IntentOutcome::ApplyFailedRolledBack {
            exec_errors,
            rollback_text,
        },
        Err(rollback_errors) => IntentOutcome::ApplyFailedRollbackFailed {
            exec_errors,
            rollback_errors,
        },
    }
}

fn validate_properties_conflicts(
    domain: Domain,
    properties: &HashMap<String, PropertyValue>,
) -> Result<(), String> {
    match domain {
        Domain::Services => service_validator::validate(properties),
    }
}

// Expects the canonical domain name — alias normalisation is the frontend's responsibility.
// The error is intentionally bare: it has no knowledge of what the frontend looks like
// or what context is relevant to show the user.
fn parse_domain(s: &str) -> Result<Domain, String> {
    match s {
        "services" => Ok(Domain::Services),
        _ => Err(format!("Unknown domain '{}'", s)),
    }
}
