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

pub struct EngineConfig {
    pub dry_run: bool,
    pub auto_approve: bool,
}

pub fn execute_order(order: Order, config: EngineConfig) -> Result<Vec<String>, Vec<String>> {
    // 1. Resolve module
    let module = module_resolver::resolve(&order.domain).map_err(|e| vec![e])?;

    match &order.action {
        Action::Config => {
            let mut output = Vec::new();

            // 2. Planning
            let plan = planner::create_plan(&module, &order);

            // Collect plan output if planning succeeded
            match &plan {
                Ok(plan_output) => output.push(plan_output.output.clone()),
                Err(_) => {}
            }

            // 3. Dry run
            if config.dry_run {
                // return what we have so far
                // return Ok(output);
            }

            // 4. Approval
            if !config.auto_approve {
                // approval hook
                // if rejected: return Ok(output);
            }

            // 5. Execution
            if let Ok(plan) = plan {
                match executor::execute_plan(&plan, &module) {
                    Ok(result_output) => output.extend(result_output),
                    Err(e) => output.extend(e),
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
