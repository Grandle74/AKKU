// engine/src/executor/mod.rs
//
// Execution layer: the only place in the engine that calls module functions.
//
// Two entry points, two responsibilities:
//   - execute_normal: dispatches imperative actions directly (no planning).
//   - execute_plan:   runs a pre-approved Plan step by step.
//
// This module never touches plan files, never makes approval decisions,
// and never builds Steps. Those concerns belong to plan_store, engine/lib.rs,
// and planner respectively.

use crate::planner::Plan;
use crate::{Order, module_resolver::ModuleId};

mod services;

/// Executes an imperative (Meta or Custom) action directly.
///
/// Called by `execute_order` for non-Config actions — no planning involved.
/// Config actions must never reach here; they go through `execute_plan`.
pub fn execute_normal(order: &Order, module_id: &ModuleId) -> Result<Vec<String>, String> {
    match module_id {
        ModuleId::Services => services::execute_services(order),
    }
}

/// Executes a pre-approved Plan step by step.
///
/// Called by `approve_plan()` in engine/lib.rs — approval logic and
/// audit-status updates live there, not here. This function has a single
/// responsibility: run the steps and return the result.
pub fn execute_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    match &plan.module_id {
        ModuleId::Services => services::execute_services_plan(plan),
    }
}
