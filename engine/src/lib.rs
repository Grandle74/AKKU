// engine/src/lib.rs
//
// Engine entry points: the two trips of the plan/approve flow.
//
// Does NOT own approval logic, run-mode semantics, or frontend rendering —
// those belong to the API layer and Frontend respectively.
//
// Invariant: all engine internals (planner, executor, plan_store, snapshot,
// module_resolver) communicate only through this file — never directly
// with each other.

pub use shared_libs::{Action, Domain, PlanSummary, PropertyValue, StepSummary};
use std::collections::HashMap;

mod executor;
mod module_resolver;
mod plan_store;
mod planner;
mod snapshot;

use planner::Plan;

// ── Core Types ────────────────────────────────────────────────────────────────

/// Describes an operation to perform, assembled by the API from parsed intent.
#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    /// Present for all actions except Meta. Required for Config & Custom — it is the plan target.
    pub target: Option<String>,
    /// Non-empty only for Config actions. Ignored by Meta/Custom dispatchers.
    pub desired_properties: HashMap<String, PropertyValue>,
    /// Set by the API: "normal" or "force". None means dry-run — do not save.
    pub mode: Option<String>,
}

/// Returned by `execute_order` for every action.
///
/// `pending_plan` is `Some` only for Config actions with at least one step.
/// The frontend must send the plan ID back via `approve_plan` after the user confirms.
/// `None` means either the action was not declarative, or the target is already
/// at the desired state.
pub struct EngineResult {
    pub output: Vec<String>,
    pub pending_plan: Option<PlanSummary>,
}

// ── Trip 1 ────────────────────────────────────────────────────────────────────

/// Dispatches an Order and returns output or a plan awaiting approval.
///
/// The engine does not distinguish run modes (dry-run, force, normal) —
/// that is the API's responsibility. The engine always returns the plan ID +
/// plan output when a plan is created; what the API does with it is the API's concern.
pub fn execute_order(order: Order) -> Result<EngineResult, Vec<String>> {
    let module = module_resolver::ModuleId::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            let maybe_plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;

            match maybe_plan {
                None => Ok(EngineResult {
                    output: vec![format!(
                        "✔ '{}' is already in the desired state — no changes needed.",
                        order.target.as_deref().unwrap_or("target")
                    )],
                    pending_plan: None,
                }),
                Some(mut plan) => {
                    if let Some(ref mode) = order.mode {
                        plan.mode = Some(mode.clone());
                        plan_store::save(&plan).map_err(|e| vec![e])?;
                    }
                    let summary = plan_store::to_summary(plan);
                    Ok(EngineResult {
                        output: vec![],
                        pending_plan: Some(summary),
                    })
                }
            }
        }

        _ => {
            let output = executor::execute_normal(&order, &module).map_err(|e| vec![e])?;
            Ok(EngineResult {
                output,
                pending_plan: None,
            })
        }
    }
}

// ── Trip 2 ────────────────────────────────────────────────────────────────────

/// Acts on a user's approval or rejection of a pending plan.
///
/// The plan file written by `execute_order` is reloaded from disk here —
/// the in-memory plan from Trip 1 is gone by the time the user confirms.
/// The file is the handoff mechanism between the two trips.
pub fn approve_plan(id: &str, approved: bool) -> Result<Vec<String>, Vec<String>> {
    if !approved {
        let _ = plan_store::update_status(id, "rejected");
        return Ok(vec!["Plan rejected.".to_string()]);
    }

    let plan = plan_store::load(id).map_err(|e| vec![e])?;

    // Rollback plans skip snapshot capture — capturing state here would overwrite
    // the pre-change snapshot with the post-change state, defeating restoration.
    // A failed snapshot saving aborts rather than warns; a plan
    // with no rollback anchor must not execute.
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

// ── Rollback Path ─────────────────────────────────────────────────────────────

/// Builds a rollback plan from a snapshot and saves it to disk without executing it.
///
/// Returns the plan ID and step descriptions for the caller to present before
/// confirmation. The caller then passes the ID to `approve_plan` — this keeps
/// the rollback on the standard approval path rather than a separate execution route.
pub fn build_rollback_plan(origin_plan_id: &str) -> Result<PlanSummary, Vec<String>> {
    let snapshot = snapshot::load(origin_plan_id).map_err(|e| vec![e])?;
    let order = snapshot.into_order().map_err(|e| vec![e])?;
    let module = module_resolver::ModuleId::resolve(&order.domain).map_err(|e| vec![e])?;
    let maybe_plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;

    let Some(mut plan) = maybe_plan else {
        return Ok(PlanSummary::empty());
    };

    plan.rollback_of = Some(origin_plan_id.to_string());
    plan.mode = Some("rollback".to_string());
    plan_store::save(&plan).map_err(|e| vec![e])?;

    Ok(plan_store::to_summary(plan))
}

/// Returns all persisted plans as ready-to-consume summaries, sorted oldest-first.
pub fn list_plans() -> Result<Vec<PlanSummary>, String> {
    plan_store::list_plans()
}
