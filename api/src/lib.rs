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

pub use engine::{Action, PlanSummary, PropertyValue, StepSummary};
use engine::{Domain, EngineResult, Order, approve_plan as engine_approve, execute_order};
use std::collections::HashMap;

mod service_validator;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the frontend to render.
pub enum IntentOutcome {
    /// Action completed immediately — returns the output lines.
    Immediate(Vec<String>),

    /// Dry-run: returns the plan structure but nothing was saved or executed.
    DryRun { plan: PlanSummary },

    /// Normal path (Trip 1): plan saved, awaiting explicit user approval.
    RequiresApproval { plan: PlanSummary },

    /// Forced path only: plan was executed immediately and succeeded.
    Applied {
        plan: PlanSummary,
        result_text: Vec<String>,
    },

    /// Forced path only: plan was executed immediately and failed.
    ApplyFailed {
        plan: PlanSummary,
        exec_errors: Vec<String>,
    },

    /// User-approved execution failed; auto-rollback succeeded.
    ApplyFailedRolledBack {
        apply_errors: Vec<String>,  // original plan failed
        result: Vec<String>,        // rollback execution output
        rollback_plan: PlanSummary, // structure — for frontends that want to render steps
    },

    /// User-approved execution failed; auto-rollback also failed.
    ApplyFailedRollbackFailed {
        apply_errors: Vec<String>,          // original plan failed
        rollback_errors: Vec<String>,       // rollback also failed
        rollback_plan: Option<PlanSummary>, // None if rollback plan could not be built; Some if built but execution failed
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
/// The returned `PlanSummary.id` is passed to `approve_intent` on confirmation,
/// giving the user a chance to review exactly what will be restored.
pub fn preview_rollback_intent(origin_plan_id: &str) -> Result<PlanSummary, Vec<String>> {
    engine::build_rollback_plan(origin_plan_id)
}

/// Returns all persisted plans as ready-to-consume summaries, sorted oldest-first.
pub fn list_plans() -> Result<Vec<PlanSummary>, String> {
    engine::list_plans()
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
    let Some(plan) = result.pending_plan else {
        return Ok(IntentOutcome::Immediate(result.output));
    };

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan }),

        RunMode::Force => match engine_approve(&plan.id, true) {
            Ok(result_text) => Ok(IntentOutcome::Applied { plan, result_text }),
            // No auto-rollback on --force. Snapshot is on disk; the plan
            // history view lets the user undo manually.
            Err(exec_errors) => Ok(IntentOutcome::ApplyFailed { plan, exec_errors }),
        },

        RunMode::Normal => Ok(IntentOutcome::RequiresApproval { plan }),
    }
}

/// Attempt auto-rollback after a user-approved execution failure and return
/// the appropriate structured outcome.
fn build_rollback_outcome(plan_id: &str, apply_errors: Vec<String>) -> IntentOutcome {
    let summary = match engine::build_rollback_plan(plan_id) {
        Err(rollback_errors) => {
            return IntentOutcome::ApplyFailedRollbackFailed {
                apply_errors,
                rollback_errors,
                rollback_plan: None,
            };
        }
        Ok(s) if s.is_empty() => {
            return IntentOutcome::ApplyFailedRolledBack {
                apply_errors,
                rollback_plan: PlanSummary::empty(),
                result: vec![
                    "Nothing to restore — pre-change state matches current state.".to_string(),
                ],
            };
        }
        Ok(s) => s,
    };

    match engine_approve(&summary.id, true) {
        Ok(result) => IntentOutcome::ApplyFailedRolledBack {
            apply_errors,
            rollback_plan: summary,
            result,
        },
        Err(rollback_errors) => IntentOutcome::ApplyFailedRollbackFailed {
            apply_errors,
            rollback_errors,
            rollback_plan: Some(summary),
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
