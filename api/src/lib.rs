// api/src/lib.rs
//
// The API layer: the single entry point from the CLI into AKKU.
//
// Responsibilities:
//   1. Parse and validate the request (domain, action, properties) — reject early.
//   2. Call the engine with a well-formed Order.
//   3. Resolve the engine's result into an IntentOutcome based on the run mode.
//   4. Forward approval decisions back to the engine (Trip 2).
//   5. Trigger auto-rollback when the user approves and execution fails (Normal path only).
//   6. Expose rollback_intent for future manual rollback from the CLI History flow.
//
// The API does NOT execute systemctl commands, does NOT build Steps,
// and does NOT persist plans — the engine handles that in execute_order.
// It also does NOT render output — that is the CLI's job.
//
// Rollback architecture:
//   Auto-rollback  — triggered HERE when approve_intent detects execution failure.
//                    Normal path only. --force fails and leaves state as-is,
//                    so the user can manually rollback later via History.
//   Manual rollback — triggered by rollback_intent(), called by the CLI after
//                     the user picks a plan from History. Not yet wired in the
//                     CLI (History is not implemented), but the full path exists.

pub use engine::Plan;
pub use engine::PropertyValue;
use engine::{Domain, Order, approve_plan as engine_approve, execute_order};
use std::collections::HashMap;

mod service_validator;

pub use engine::Action;
use engine::EngineResult;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the CLI to render.
pub enum IntentOutcome {
    /// Action completed immediately — display the output lines.
    Immediate(Vec<String>),

    /// Dry-run: plan was generated but nothing was saved or executed.
    DryRun { plan_text: Vec<String> },

    /// Normal flow: plan saved, awaiting explicit user approval.
    RequiresApproval { plan_id: String },

    /// Plan was approved (auto or manual) and executed successfully.
    Applied {
        plan_id: String,
        result_text: Vec<String>,
    },

    /// Execution failed. --force only — state left as-is for manual rollback.
    ApplyFailed {
        plan_id: String,
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
                mode: None,
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
        mode: match (is_config, mode) {
            (true, RunMode::Normal) => Some("normal".to_string()),
            (true, RunMode::Force) => Some("force".to_string()),
            _ => None, // DryRun or non-Config: don't save
        },
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

/// Preview-only rollback: generates and saves the rollback plan without executing it.
///
/// Called by the History TUI on the first Enter. The returned Plan is displayed
/// to the user in the popup, then passed to `approve_intent` on confirmation.
/// This gives the user a chance to see exactly what will be restored before committing.
pub fn preview_rollback_intent(origin_plan_id: &str) -> Result<(String, Vec<String>), Vec<String>> {
    engine::preview_rollback_plan(origin_plan_id)
}

pub fn read_plan(id: &str) -> Result<Vec<String>, String> {
    engine::read_plan(id)
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
fn resolve_outcome(result: EngineResult, mode: &RunMode) -> Result<IntentOutcome, Vec<String>> {
    let plan_text = result.output;

    // Engine returned None: the service is already at desired state.
    let Some(plan_id) = result.pending_plan else {
        return Ok(IntentOutcome::Immediate(plan_text));
    };

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan_text }),

        // No auto-rollback on --force. The snapshot is on disk; if execution
        // fails, the user decides what to do next via History.
        RunMode::Force => match engine_approve(&plan_id, true) {
            Ok(result_text) => Ok(IntentOutcome::Applied {
                plan_id,
                result_text,
            }),
            Err(exec_errors) => Ok(IntentOutcome::ApplyFailed {
                plan_id,
                exec_errors,
            }),
        },

        RunMode::Normal => Ok(IntentOutcome::RequiresApproval { plan_id }),
    }
}

/// Attempts auto-rollback after a user-approved execution failure and returns
/// the appropriate structured outcome.
///
/// Only called by approve_intent — never by the Force path.
/// plan_text is intentionally NOT threaded through — the CLI already rendered
/// the plan before the approval prompt, so including it would cause a double-print.
fn build_rollback_outcome(plan_id: &str, exec_errors: Vec<String>) -> IntentOutcome {
    match engine_approve(&plan_id, true) {
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

pub fn list_plans() -> Result<Vec<Plan>, String> {
    engine::list_plans()
}

// ── ID parsing ────────────────────────────────────────────────────────────────

/// Extracts the human-readable date portion from a plan ID.
///
/// ID format: `<prefix>_<YYYYMMDD>_<HHMMSS>_<hex>`
/// Example:   `svc_20260407_143022_a3f2`  →  `2026-04-07 14:30`
pub fn date_from_id(id: &str) -> String {
    let parts: Vec<&str> = id.split('_').collect();

    // A well-formed ID has at least 4 segments: prefix, date, time, hex.
    if parts.len() < 4 {
        return id.to_string();
    }

    let date = parts[1]; // YYYYMMDD
    let time = parts[2]; // HHMMSS

    if date.len() == 8 && time.len() == 6 {
        format!(
            "{}-{}-{} {}:{}",
            &date[0..4],
            &date[4..6],
            &date[6..8],
            &time[0..2],
            &time[2..4],
        )
    } else {
        id.to_string()
    }
}

// ── Touched targets ────────────────────────────────────────────────────────

/// Returns a warning string if any plan *after* the selected one in the
/// sorted list also completed changes on the same target.
///
/// "After" is defined by position in the sorted list (newest-last),
/// meaning all entries with a higher index than `selected`.
///
/// Returns None when it is safe to proceed without a warning.
pub fn plans_after_touching_target(plan_id: &str) -> Result<usize, String> {
    let plans = engine::list_plans()?;
    let pos = plans
        .iter()
        .position(|p| p.id == plan_id)
        .ok_or_else(|| format!("Plan '{}' not found", plan_id))?;
    let target = &plans[pos].target;
    let count = plans[pos + 1..]
        .iter()
        .filter(|p| p.target == *target && p.status == "completed")
        .count();
    Ok(count)
}
