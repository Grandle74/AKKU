use crate::{Order, PropertyValue, module_resolver::ModuleId};
use shared_libs::Steps;
use std::collections::HashMap;

pub struct Plan {
    pub target: String,
    pub output: String,
    pub steps: Steps,
}

pub fn create_plan(module: &ModuleId, order: &Order) -> Result<Plan, String> {
    // planner extracts from Order
    let target = order.target.clone().ok_or("No target")?;
    let props = &order.desired_properties;

    let steps: Steps = match module {
        ModuleId::Services => plan_services(target.clone(), props)?,
        // ModuleId::Network => { ... }
    };

    // Shall be styled later...
    let output = format!(
        "=====Plan for '{}':=====\n{}\n==========================",
        target,
        steps
            .iter()
            .map(|s| s.description.clone())
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(Plan {
        target,
        output,
        steps,
    })
}

fn plan_services(target: String, props: &HashMap<String, PropertyValue>) -> Result<Steps, String> {
    // Get States
    let current_state = services::state_helpers::ServiceCurrentState::new(&target)?;
    let desired_state = services::state_helpers::ServiceDesiredState::from_props(target, props)?;
    // passes primitives to module
    let delta = services::state_helpers::calc(&current_state, &desired_state);
    Ok(services::state_helpers::to_steps(&delta))
}
