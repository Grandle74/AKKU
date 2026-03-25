pub use shared_libs::{Action, Domain, PropertyValue};
use std::collections::HashMap;

mod executor;
pub mod module_resolver;
mod plan_store;
mod planner;

pub use module_resolver::ModuleId;
pub use planner::Plan;

// ── Core Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub target: Option<String>,
    pub desired_properties: HashMap<String, PropertyValue>,
}

/// Returned by execute_order for every action.
/// `pending_plan` is Some only for Config actions — it must be sent back
/// via approve_plan() after the user confirms. All other actions leave it None.
pub struct EngineResult {
    pub output: Vec<String>,
    pub pending_plan: Option<Plan>,
}

// ── Engine Entry Points ─────────────────────────────────────────────────────

/// Trip 1 — Process an order.
/// For Config actions: plans, saves to disk, returns pending plan for approval.
/// For all other actions: executes immediately, no approval needed.
pub fn execute_order(order: Order, dry_run: bool) -> Result<EngineResult, Vec<String>> {
    let module = module_resolver::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            // Planning: queries current state, diffs against desired, builds steps.
            // Also persists to ~/.yast3/plans/<id>.plan.json for audit trail.
            let plan = planner::create_plan(&module, &order).map_err(|e| vec![e])?;

            let mut output = Vec::new();
            output.push(plan.output.clone()); // formatted step list shown to user

            if dry_run {
                // Dry run stops here — plan is shown, nothing is executed or approved.
                return Ok(EngineResult {
                    output,
                    pending_plan: None,
                });
            }

            // Hand plan back to the caller (API → frontend).
            // Frontend will show it, ask approval, then call approve_plan().
            Ok(EngineResult {
                output,
                pending_plan: Some(plan),
            })
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

/// Trip 2 — Act on a pending plan after user decision.
/// Takes the in-memory Plan returned from execute_order — no file reload needed.
/// Updates the plan file status at each stage for audit/rollback purposes.
pub fn approve_plan(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    if !approved {
        // Persist rejection — audit trail, rollback system can skip this plan safely.
        plan_store::update_status(&plan.id, "rejected").map_err(|e| vec![e])?;
        return Ok(vec!["Plan rejected.".to_string()]);
    }

    // Mark executing before step 1 — if process crashes mid-flight, file reflects that.
    plan_store::update_status(&plan.id, "executing").map_err(|e| vec![e])?;

    // Plan carries its own module_id — executor dispatches without needing it passed separately.
    let result = executor::execute_plan(&plan);

    // Always update final status — clean audit trail regardless of outcome.
    match &result {
        Ok(_) => plan_store::update_status(&plan.id, "completed").ok(),
        Err(_) => {
            plan_store::update_status(&plan.id, "failed").ok()
            // Rollback hook: passes plan.id to rollback logic — not implemented yet.
        }
    };

    result
}

/// Standalone plan inspection — used for dry-run or debugging without approval flow.
pub fn plan(order: Order) -> Result<Plan, Vec<String>> {
    planner::create_plan(
        &module_resolver::resolve(&order.domain).map_err(|e| vec![e])?,
        &order,
    )
    .map_err(|e| vec![e])
}
