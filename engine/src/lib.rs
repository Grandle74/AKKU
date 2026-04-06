// engine/src/lib.rs
pub use shared_libs::{Action, Domain, PropertyValue};
use std::collections::HashMap;

mod executor;
pub mod module_resolver;
mod plan_store;
mod planner;

pub use module_resolver::ModuleId;
pub use planner::Plan;

// ── Core Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub target: Option<String>,
    pub desired_properties: HashMap<String, PropertyValue>,
}

/// Returned by `execute_order` for every action.
/// `pending_plan` is `Some` only for Config actions — the frontend must send it
/// back via `approve_plan()` after the user confirms. All other actions return `None`.
pub struct EngineResult {
    pub output: Vec<String>,
    pub pending_plan: Option<Plan>,
}

// ── Engine Entry Points ──────────────────────────────────────────────────────

/// Trip 1 — Process an Order.
///
/// - Config actions: plans, saves to disk, returns a pending Plan for approval.
/// - All other actions: execute immediately, no approval needed.
pub fn execute_order(order: Order, dry_run: bool) -> Result<EngineResult, Vec<String>> {
    let module = module_resolver::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            // Query current state, diff against desired, build ordered steps.
            // Persists to ~/.yast3/plans/<id>.plan.json for audit trail.
            let plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;
            let output = vec![plan.output.clone()];

            // No steps = already at desired state. Nothing to approve, nothing was saved.
            if plan.steps.is_empty() {
                // Dry run: show plan, execute nothing.
                return Ok(EngineResult {
                    output,
                    pending_plan: None,
                });
            }

            if dry_run {
                // Hand plan back to caller (API → frontend) for approval.
                Ok(EngineResult {
                    output,
                    pending_plan: Some(plan),
                })
            } else {
                Ok(EngineResult {
                    output,
                    pending_plan: None,
                })
            }
        }

        _ => {
            // Imperative actions (list, status, start, ...) — direct execution, no planning.
            let output = executor::execute_normal(&order, &module).map_err(|e| vec![e])?;
            Ok(EngineResult {
                output,
                pending_plan: None,
            })
        }
    }
}

/// Trip 2 — Act on a pending Plan after the user's decision.
///
/// Takes the in-memory Plan returned from `execute_order` — no file reload needed.
/// Updates the plan file status at each stage for audit/rollback purposes.
pub fn approve_plan(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    if !approved {
        // Persist rejection so the audit trail and rollback system can skip this plan.
        plan_store::update_status(&plan.id, "rejected").map_err(|e| vec![e])?;
        return Ok(vec!["Plan rejected.".to_string()]);
    }

    // Mark as executing before step 1 — if the process crashes mid-flight, the file reflects it.
    plan_store::update_status(&plan.id, "executing").map_err(|e| vec![e])?;

    let result = executor::execute_plan(&plan);

    // Always update the final status — clean audit trail regardless of outcome.
    match &result {
        Ok(_) => plan_store::update_status(&plan.id, "completed").ok(),
        Err(_) => {
            plan_store::update_status(&plan.id, "failed").ok()
            // TODO: Rollback hook — pass plan.id to rollback logic (not implemented yet).
        }
    };

    result
}
