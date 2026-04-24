// api/src/lib.rs
//
// The API layer: the single entry point from the CLI into YaST3.
//
// Responsibilities:
//   1. Parse and validate the request (domain, action, properties) — reject early.
//   2. Call the engine with a well-formed Order.
//   3. Resolve the engine's result into an IntentOutcome based on the run mode.
//   4. Forward approval decisions back to the engine (Trip 2).
//   5. Trigger auto-rollback when the user approves and execution fails (Normal path only).
//   6. Expose rollback_intent for future manual rollback from the CLI History flow.
//
// The API does NOT execute systemctl commands and does NOT build Steps.
// It also does NOT render output — that is the CLI's job.
//
// Rollback architecture:
//   Auto-rollback  — triggered HERE when approve_intent detects execution failure.
//                    Normal path only. --force fails and leaves state as-is,
//                    so the user can manually rollback later via History.
//   Manual rollback — triggered by rollback_intent(), called by the CLI after
//                     the user picks a plan from History. Not yet wired in the
//                     CLI (History is not implemented), but the full path exists.

use engine::{Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

pub use engine::Action;
pub use engine::EngineResult as IntentResult;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the CLI to render.
pub enum IntentOutcome {
    /// Action completed immediately — display the output lines.
    Immediate(Vec<String>),

    /// Dry-run: plan was generated but nothing was saved or executed.
    DryRun { plan_text: Vec<String> },

    /// Normal flow: plan saved, awaiting explicit user approval.
    RequiresApproval { plan: Plan, plan_text: Vec<String> },

    /// Plan was approved (auto or manual) and executed successfully.
    Applied {
        plan_text: Vec<String>,
        result_text: Vec<String>,
    },

    /// Execution failed. --force only — state left as-is for manual rollback.
    ApplyFailed {
        plan_text: Vec<String>,
        exec_errors: Vec<String>,
    },

    /// User-approved execution failed and auto-rollback restored previous state.
    ///
    /// No plan_text — the CLI already rendered the plan before the approval prompt.
    /// rollback_text is raw executor output; the CLI owns the rollback header.
    ApplyFailedRolledBack {
        exec_errors: Vec<String>,
        rollback_text: Vec<String>,
    },

    /// User-approved execution failed and auto-rollback also failed.
    ///
    /// No plan_text — same reason as ApplyFailedRolledBack.
    ApplyFailedRollbackFailed {
        exec_errors: Vec<String>,
        rollback_errors: Vec<String>,
    },

    /// Manual rollback completed successfully.
    /// rollback_text is raw executor output; the CLI owns the rollback plan header.
    RolledBack {
        origin_plan_id: String,
        rollback_text: Vec<String>,
    },

    /// Manual rollback itself failed.
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

    let is_config = matches!(action, Action::Config);
    let result = execute_order(Order {
        domain,
        action,
        target: Some(target),
        desired_properties: properties,
    })?;

    // Non-Config actions always execute immediately — the engine returns no plan.
    if !is_config {
        return Ok(IntentOutcome::Immediate(result.output));
    }

    resolve_outcome(result, mode)
}

/// Trip 2: forwards a user's approval decision to the engine.
///
/// On execution failure the API — not the engine — decides to roll back.
/// Auto-rollback is the Normal path's safety net. --force has no auto-rollback
/// by design: the user asserted control, the snapshot is on disk, History will
/// let them undo manually.
///
/// Returns Ok with success lines on approval + successful execution,
/// or Ok with "Plan rejected." on rejection (not an error — user chose this),
/// or Err(IntentOutcome) on execution failure so the CLI can render the
/// correct structured outcome without any string-parsing on its end.
pub fn approve_intent(plan: Plan, approved: bool) -> Result<Vec<String>, IntentOutcome> {
    if !approved {
        // engine_approve handles the plan_store status update for rejected plans.
        // Rejection always returns Ok("Plan rejected.") — the unwrap is safe.
        return Ok(
            engine_approve(plan, false).unwrap_or_else(|_| vec!["Plan rejected.".to_string()])
        );
    }

    let plan_id = plan.id.clone();

    match engine_approve(plan, true) {
        Ok(output) => Ok(output),
        Err(exec_errors) => Err(build_rollback_outcome(&plan_id, exec_errors)),
    }
}

/// Manual rollback entry point: restores a target to its pre-execution state
/// using the snapshot captured before the original plan ran.
///
/// Called by the CLI History flow once implemented — NOT called during normal
/// approve/execute cycles. The plan_id comes from the user selecting a past plan.
pub fn rollback_intent(origin_plan_id: &str) -> IntentOutcome {
    match engine::rollback_plan(origin_plan_id) {
        Ok(rollback_text) => IntentOutcome::RolledBack {
            origin_plan_id: origin_plan_id.to_string(),
            rollback_text,
        },
        Err(errors) => IntentOutcome::RollbackFailed {
            origin_plan_id: origin_plan_id.to_string(),
            errors,
        },
    }
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
            save_plan(&plan)?;
            // No auto-rollback on --force. The snapshot is on disk; if execution
            // fails, the user decides what to do next via History.
            match engine_approve(plan, true) {
                Ok(result_text) => Ok(IntentOutcome::Applied {
                    plan_text,
                    result_text,
                }),
                Err(exec_errors) => Ok(IntentOutcome::ApplyFailed {
                    plan_text,
                    exec_errors,
                }),
            }
        }

        RunMode::Normal => {
            // Save before returning to the frontend — guarantees an audit record
            // exists even if the process is killed during the approval window.
            save_plan(&plan)?;
            Ok(IntentOutcome::RequiresApproval { plan, plan_text })
        }
    }
}

/// Attempts auto-rollback after a user-approved execution failure and returns
/// the appropriate structured outcome.
///
/// Only called by approve_intent — never by the Force path.
/// plan_text is intentionally NOT threaded through — the CLI already rendered
/// the plan before the approval prompt, so including it would cause a double-print.
fn build_rollback_outcome(plan_id: &str, exec_errors: Vec<String>) -> IntentOutcome {
    match engine::rollback_plan(plan_id) {
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

/// Persists the plan via the engine's public surface.
///
/// The API never touches plan_store directly — that is a private engine
/// implementation detail. Extracted as a helper to avoid repeating the
/// map_err pattern across run mode branches.
fn save_plan(plan: &Plan) -> Result<(), Vec<String>> {
    engine::save_plan(plan).map_err(|e| vec![e])
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
