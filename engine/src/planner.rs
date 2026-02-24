// engine/src/planner.rs
use crate::{Action, Domain, Order, PropertyValue};
use services::current_state::ServiceState;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub description: String,
}

pub struct ExecutionPlan {
    pub steps: Vec<Step>,
}

pub fn create_plan(order: Order) -> Result<ExecutionPlan, String> {
    match order.domain {
        Domain::Services => create_service_plan(&order),
    }
}

fn create_service_plan(order: &Order) -> Result<ExecutionPlan, String> {
    let current = ServiceState::new(&order.target)?;
    let diff = calculate_diff(&current, &order.desired_properties);
    let steps = generate_steps(&diff, &order.target, order.domain.clone());

    Ok(ExecutionPlan { steps })
}

struct StateDiff {
    needs_start: bool,
    needs_stop: bool,
    needs_enable: bool,
    needs_disable: bool,
    needs_mask: bool,
    needs_unmask: bool,
}

fn calculate_diff(current: &ServiceState, desired: &HashMap<String, PropertyValue>) -> StateDiff {
    if current.masked {
        // Should return an error result -- needs to be fixed
        // This also doesn't allow unmasking and enabling/starting/stopping/disabling in the same time - another issue
        StateDiff {
            needs_start: false,
            needs_stop: false,
            needs_enable: false,
            needs_disable: false,
            needs_mask: get_bool(desired, "masked") == Some(true) && !current.masked,
            needs_unmask: get_bool(desired, "masked") == Some(false) && current.masked,
        }
    } else {
        StateDiff {
            needs_start: get_bool(desired, "running") == Some(true) && !current.active,
            needs_stop: get_bool(desired, "running") == Some(false) && current.active,
            needs_enable: get_bool(desired, "enabled") == Some(true) && !current.enabled,
            needs_disable: get_bool(desired, "enabled") == Some(false) && current.enabled,
            needs_mask: get_bool(desired, "masked") == Some(true) && !current.masked,
            needs_unmask: get_bool(desired, "masked") == Some(false) && current.masked,
        }
    }
}

fn generate_steps(diff: &StateDiff, service: &str, domain: Domain) -> Vec<Step> {
    let mut steps = Vec::new();

    // Order matters! Unmask before enable, enable before start

    if diff.needs_unmask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Unmask,
            target: service.to_string(),
            description: format!("Unmask {}", service),
        });
    }

    if diff.needs_enable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Enable,
            target: service.to_string(),
            description: format!("Enable {}", service),
        });
    }

    if diff.needs_disable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Disable,
            target: service.to_string(),
            description: format!("Disable {}", service),
        });
    }

    if diff.needs_start {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Start,
            target: service.to_string(),
            description: format!("Start {}", service),
        });
    }

    if diff.needs_stop {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Stop,
            target: service.to_string(),
            description: format!("Stop {}", service),
        });
    }

    if diff.needs_mask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Mask,
            target: service.to_string(),
            description: format!("Mask {}", service),
        });
    }

    steps
}

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    match props.get(key) {
        Some(PropertyValue::Bool(b)) => Some(*b),
        _ => None,
    }
}
