// engine/src/lib.rs
pub use shared_libs::{Action, Domain, PropertyValue};
use std::collections::HashMap;

mod executor;
pub mod module_resolver;
mod planner;

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

pub fn execute_order(order: Order, config: EngineConfig) -> Result<(), String> {
    // 1. Resolve module
    let module = module_resolver::resolve(&order.domain)?;

    match &order.action {
        Action::Config => {
            // 2. Planning
            let plan = planner::create_plan(&module, &order)?;

            // 3. Dry Run
            if config.dry_run {
                return Ok(());
            }

            // 4. Approval
            if !config.auto_approve {
                // approval layer hook (UI/CLI)
                // if rejected _ return Ok(())
            } else {
                // 5. Execution
                // executor::execute_plan(module.as_ref(), &plan)?;
                executor::execute(&order, module)?;
            }
        }
        _ => {
            // Normal execution
            executor::execute(&order, module)?;
        }
    }

    Ok(())
}
