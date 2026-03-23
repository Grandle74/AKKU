// engine/src/planner.rs
use crate::{Action, Domain, Order, PropertyValue};
use services::current_state::ServiceState;
use std::collections::HashMap;

pub struct ExecutionPlan {
    pub steps: Vec<Step>,
}

#[derive(Debug)]
pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub description: String,
}

pub fn create_plan(order: Order) -> Result<ExecutionPlan, String> {
    match order.domain {
        Domain::Services => create_service_plan(&order),
    }
}

// THIS IS A HELPER STRUCT --- MEANS IT IS MODULAR
struct StateDelta {
    needs_start: bool,
    needs_stop: bool,
    needs_enable: bool,
    needs_disable: bool,
    needs_mask: bool,
    needs_unmask: bool,
}
impl StateDelta {
    fn calculate_delta(
        current: &ServiceState,
        desired: &HashMap<String, PropertyValue>,
    ) -> StateDelta {
        StateDelta {
            needs_start: StateDelta::get_bool(desired, "running") == Some(true) && !current.active,
            needs_stop: StateDelta::get_bool(desired, "running") == Some(false) && current.active,
            needs_enable: StateDelta::get_bool(desired, "enabled") == Some(true)
                && !current.enabled,
            needs_disable: StateDelta::get_bool(desired, "enabled") == Some(false)
                && current.enabled,
            needs_mask: StateDelta::get_bool(desired, "masked") == Some(true) && !current.masked,
            needs_unmask: StateDelta::get_bool(desired, "masked") == Some(false) && current.masked,
        }
    }

    fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
        match props.get(key) {
            Some(PropertyValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }
}

// THE FOLLOWING FUNCTIONS ARE HELPER FUNCTIONS --- ALSO MODULAR
fn create_service_plan(order: &Order) -> Result<ExecutionPlan, String> {
    let current = ServiceState::new(&order.target)?; // It's Defined in the Service module
    let delta = StateDelta::calculate_delta(&current, &order.desired_properties); // It's located here
    let steps = generate_service_steps(&delta, &order.target, order.domain.clone()); // It's located here

    Ok(ExecutionPlan { steps })
}
fn generate_service_steps(delta: &StateDelta, service: &str, domain: Domain) -> Vec<Step> {
    let mut steps = Vec::new();

    // Order matters! Unmask before enable, enable before start

    if delta.needs_unmask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Unmask,
            target: service.to_string(),
            description: format!("Unmask {}", service),
        });
    }

    if delta.needs_enable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Enable,
            target: service.to_string(),
            description: format!("Enable {}", service),
        });
    }

    if delta.needs_disable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Disable,
            target: service.to_string(),
            description: format!("Disable {}", service),
        });
    }

    if delta.needs_start {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Start,
            target: service.to_string(),
            description: format!("Start {}", service),
        });
    }

    if delta.needs_stop {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Stop,
            target: service.to_string(),
            description: format!("Stop {}", service),
        });
    }

    if delta.needs_mask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Mask,
            target: service.to_string(),
            description: format!("Mask {}", service),
        });
    }

    steps
}
