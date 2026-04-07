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
pub mod module_resolver;
mod plan_store;
mod planner;

pub use module_resolver::ModuleId;
pub use planner::Plan;

/// Public wrapper so the API layer can persist a plan without importing
/// plan_store directly. plan_store stays a private engine implementation detail.
pub fn save_plan(plan: &Plan) -> Result<(), String> {
    plan_store::save(plan)
}

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
}

/// Returned by `execute_order` for every action.
///
/// `pending_plan` is `Some` only for Config actions with at least one step —
/// the frontend must send it back via `approve_plan()` after the user confirms.
/// `None` means either the action was not declarative, or the service is already
/// at the desired state (no steps needed).
pub struct EngineResult {
    pub output: Vec<String>,
    pub pending_plan: Option<Plan>,
}

// ── Trip 1 ────────────────────────────────────────────────────────────────────

/// Processes an Order and returns a result or a plan requiring approval.
///
/// The engine does not know about run modes (dry-run, force, normal).
/// That distinction is the API layer's responsibility. The engine always
/// returns the Plan when one is created — what the API does with it is
/// the API's concern.
pub fn execute_order(order: Order) -> Result<EngineResult, Vec<String>> {
    let module = module_resolver::resolve(&order.domain).map_err(|e| vec![e])?;

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
                Some(plan) => {
                    // Hand the plan's display lines to the frontend as-is.
                    // The frontend decides how to render them.
                    let output = plan.output.clone();
                    Ok(EngineResult {
                        output,
                        pending_plan: Some(plan),
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

// ── Trip 2 ───────────────────────────────────────────────────────────────────

/// Acts on a user's approval or rejection of a pending Plan.
///
/// Takes the in-memory Plan that was returned from `execute_order` —
/// no file reload is needed for execution. The plan file (written by
/// `engine::save_plan` in the API layer before this is called) is only
/// updated here for audit-trail purposes.
pub fn approve_plan(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    if !approved {
        // Best-effort status update — a rejection is recorded for the audit trail
        // but we don't surface a file-write error to the user for a rejection.
        let _ = plan_store::update_status(&plan.id, "rejected");
        return Ok(vec!["Plan rejected.".to_string()]);
    }

    // Mark as executing BEFORE the first step. If the process crashes mid-flight,
    // the file reflects "executing" rather than "pending", aiding diagnosis.
    plan_store::update_status(&plan.id, "executing").map_err(|e| vec![e])?;

    let result = executor::execute_plan(&plan);

    // Always record the final outcome — a clean audit trail is non-negotiable.
    match &result {
        Ok(_) => {
            let _ = plan_store::update_status(&plan.id, "completed");
        }
        Err(_) => {
            let _ = plan_store::update_status(&plan.id, "failed");
            // TODO: Rollback hook — pass plan.id to rollback logic.
        }
    }

    result
}
