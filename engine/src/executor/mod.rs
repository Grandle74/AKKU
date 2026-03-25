use crate::planner::Plan;
use crate::{Order, module_resolver::ModuleId};

mod services;

// ── Public Entry Points ──────────────────────────────────────────────────────

/// Executes imperative actions directly (list, status, start, ...).
/// Called by execute_order for non-Config actions — no planning involved.
pub fn execute_normal(order: &Order, module_id: &ModuleId) -> Result<Vec<String>, String> {
    match module_id {
        ModuleId::Services => services::execute_services(order),
    }
}

/// Executes a pre-approved plan step by step.
/// Called by approve_plan() in engine/lib.rs — approval and status tracking live there.
/// This function only runs steps and returns the result.
pub fn execute_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    match &plan.module_id {
        ModuleId::Services => services::execute_services_plan(plan),
    }
}
