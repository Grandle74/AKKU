// engine/src/lib.rs
pub use shared_libs::{Action, Domain, PropertyValue};
use std::collections::HashMap;

mod executor;
pub mod module_resolver;
mod planner;

pub use planner::Plan;

//
// ── Core Types ─────────────────────────────────────────────────────────
//

#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub target: Option<String>,
    pub desired_properties: HashMap<String, PropertyValue>,
}

// ── Engine Entry ───────────────────────────────────────────────────────

pub fn execute_order(order: Order, dry_run: bool) -> Result<Vec<String>, Vec<String>> {
    // 1. Resolve module
    let module = module_resolver::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            let mut output = Vec::new();

            // 2. Planning
            let plan = plan(order);

            // Collect plan output if planning succeeded
            match &plan {
                Ok(plan_output) => output.push(plan_output.output.clone()),
                Err(_) => return Err(vec!["Failed to plan".to_string()]),
            }

            // 3. Dry run
            if dry_run {
                // return what we have so far
                return Ok(output);
            }

            // 5. Execution
            if let Ok(plan) = plan {
                match executor::execute_plan(&plan, &module) {
                    Ok(result_output) => output.extend(result_output),
                    Err(e) => return Err(e),
                }
            }

            Ok(output)
        }

        _ => {
            // Normal execution
            executor::execute(&order, &module).map_err(|e| vec![e])
        }
    }
}

pub fn plan(order: Order) -> Result<Plan, Vec<String>> {
    planner::create_plan(
        &module_resolver::resolve(&order.domain).map_err(|e| vec![e])?,
        &order,
    )
    .map_err(|e| vec![e])
}
