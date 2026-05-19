// engine/src/executor/mod.rs
//
// The only place in the engine that calls module functions.
//
// Does NOT build Steps, touch plan files, or make approval decisions —
// those belong to planner, plan_store, and engine/lib.rs respectively.

use crate::planner::Plan;
use crate::{Order, module_resolver::ModuleId};

mod services;

/// Dispatches an imperative (Meta or Custom) action directly.
///
/// Config actions must never reach here — they are planned and approved
/// before execution, and arrive via `execute_plan` instead.
pub fn execute_normal(order: &Order, module_id: &ModuleId) -> Result<Vec<String>, String> {
    match module_id {
        ModuleId::Services => services::execute_services(order),
    }
}

/// Executes a pre-approved Plan step by step.
///
/// Approval logic and audit-status updates live in `engine/lib.rs`.
/// This function's sole responsibility is running the steps and returning the result.
pub fn execute_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    match &plan.module_id {
        ModuleId::Services => services::execute_services_plan(plan),
    }
}
