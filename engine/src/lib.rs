// engine/src/lib.rs
//
// Engine entry points: the two trips of the Plan/approve flow.
//
// Trip 1 — `execute_order`: dispatches an Order.
//   - Meta/Custom actions execute immediately and return output.
//   - Config actions plan and return the Plan for the API to handle.
//     The engine has no opinion on dry-run vs force vs normal — that is
//     the API layer's concern. The engine ALWAYS returns the plan.
//
// Trip 2 — `approve_plan`: acts on a user's yes/no decision.
//   Executes or rejects the plan and updates the audit file.

pub use shared_libs::{Action, Domain, PropertyValue};
use std::collections::HashMap;

mod executor;
mod module_resolver;
mod plan_store;
mod planner;
mod snapshot;

use planner::Plan;

// ── Core Types ────────────────────────────────────────────────────────────────

/// Describes an operation to perform, as assembled by the API from parsed intent.
#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    /// Present for all actions except Meta. Required for Config (it is the plan target).
    pub target: Option<String>,
    /// Non-empty only for Config actions. Ignored by Meta/Custom dispatchers.
    pub desired_properties: HashMap<String, PropertyValue>,
    /// Set by the API to "normal", "force", or "rollback". None means dry-run — do not save.
    pub mode: Option<String>,
}

/// Returned by `execute_order` for every action.
///
/// `pending_plan` is `Some` only for Config actions with at least one step —
/// the frontend must send it back via `approve_plan()` after the user confirms.
/// `None` means either the action was not declarative, or the service is already
/// at the desired state (no steps needed).
pub struct EngineResult {
    pub output: Vec<String>,
    pub pending_plan: Option<String>,
}

// ── Trip 1 ────────────────────────────────────────────────────────────────────

/// Processes an Order and returns a result or a plan requiring approval.
///
/// The engine does not know about run modes (dry-run, force, normal).
/// That distinction is the API layer's responsibility. The engine always
/// returns the Plan when one is created — what the API does with it is
/// the API's concern.
pub fn execute_order(order: Order) -> Result<EngineResult, Vec<String>> {
    let module = module_resolver::ModuleId::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            let maybe_plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;

            match maybe_plan {
                None => {
                    // Already at desired state — no plan, no approval needed.
                    let msg = format!(
                        "✔ '{}' is already in the desired state — no changes needed.",
                        order.target.as_deref().unwrap_or("target")
                    );
                    Ok(EngineResult {
                        output: vec![msg],
                        pending_plan: None,
                    })
                }
                Some(mut plan) => {
                    // Save to disk when a mode is set. None means dry-run — no audit record.
                    if let Some(ref mode) = order.mode {
                        plan.mode = Some(mode.clone());
                        plan_store::save(&plan).map_err(|e| vec![e])?;
                    }
                    // Hand the plan's display lines to the frontend as-is.
                    // The frontend decides how to render them.
                    let output = plan.output.clone();
                    let plan_id = plan.id.clone();
                    Ok(EngineResult {
                        output,
                        pending_plan: Some(plan_id),
                    })
                }
            }
        }

        _ => {
            // Meta and Custom actions execute immediately — no planning, no approval.
            let output = executor::execute_normal(&order, &module).map_err(|e| vec![e])?;
            Ok(EngineResult {
                output,
                pending_plan: None,
            })
        }
    }
}

// ── Trip 2 (Normal) ───────────────────────────────────────────────────────────────────

/// Acts on a user's approval or rejection of a pending Plan.
///
/// Takes the in-memory Plan that was returned from `execute_order` —
/// no file reload is needed for execution. The plan file (written by
/// `execute_order` before returning) is only updated here for audit-trail purposes.
pub fn approve_plan(id: &str, approved: bool) -> Result<Vec<String>, Vec<String>> {
    if !approved {
        let _ = plan_store::update_status(id, "rejected");
        return Ok(vec!["Plan rejected.".to_string()]);
    }

    let plan = plan_store::load(id).map_err(|e| vec![e])?;

    if plan.rollback_of.is_none()
        && let Err(e) = snapshot::save(&plan.id, &plan.module_id.to_domain(), &plan.target)
    {
        let _ = plan_store::update_status(id, "aborted");
        return Err(vec![e]);
    }

    plan_store::update_status(id, "executing").map_err(|e| vec![e])?;

    let result = executor::execute_plan(&plan);

    match &result {
        Ok(_) => {
            let _ = plan_store::update_status(id, "completed");
        }
        Err(_) => {
            let _ = plan_store::update_status(id, "failed");
        }
    }

    result
}

// ── Direct Trip (Rollback) ─────────────────────────────────────────────────────────

/// Generates a rollback plan from a snapshot and saves it to disk,
/// but does NOT execute it.
///
/// Called by the History TUI to show the user what will be restored
/// before they confirm. The returned Plan is passed to `approve_plan`
/// on the user's second Enter.
pub fn preview_rollback_plan(origin_plan_id: &str) -> Result<(String, Vec<String>), Vec<String>> {
    let snapshot = snapshot::load(origin_plan_id).map_err(|e| vec![e])?;
    let order = snapshot.into_order().map_err(|e| vec![e])?;
    let module = module_resolver::ModuleId::resolve(&order.domain).map_err(|e| vec![e])?;
    let maybe_plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;

    let Some(mut plan) = maybe_plan else {
        return Err(vec![
            "Target is already at the pre-execution state — nothing to restore.".to_string(),
        ]);
    };

    plan.rollback_of = Some(origin_plan_id.to_string());
    plan.mode = Some("rollback".to_string());
    plan_store::save(&plan).map_err(|e| vec![e])?;

    let descriptions = plan.steps.iter().map(|s| s.description.clone()).collect();
    Ok((plan.id, descriptions))
}

/// Restores a target to its pre-execution state by loading the snapshot
/// captured before the original plan ran.
///
/// Executes immediately without user approval — mirrors --force behavior.
/// No snapshot is taken before this execution (rollback_of is Some).
/// Used by the auto-rollback path only — the History TUI uses
/// preview_rollback_plan + approve_plan instead.
pub fn rollback_plan(origin_plan_id: &str) -> Result<Vec<String>, Vec<String>> {
    let (plan_id, _) = preview_rollback_plan(origin_plan_id)?;
    approve_plan(&plan_id, true)
}

pub fn read_plan(id: &str) -> Result<Vec<String>, String> {
    plan_store::read(id)
}
