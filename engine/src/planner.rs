use crate::{Action, Domain, Order};
use services::current_state::ServiceState;

#[derive(Debug)]
pub struct Step {
    domain: Domain,
    action: Action,
    target: String,
    description: String,
}

pub fn create_plan(order: Order) -> Result<Vec<Step>, String> {
    let target = order.arguments.as_ref().unwrap()[0].clone();
    // Get the current state of the System/Target
    let current_state = ServiceState::new(&target)?;

    // Get the Desired State given by Order
    // Currently is not Declarative - Needs Update after API changes
    let desired_state = ServiceState {
        name: target.clone(),
        active: true,
        enabled: true,
        masked: true,
    };

    // Calculate the difference between the current and desired states
    let diff = StateDiff::calculate_diff(&current_state, &desired_state);

    // Future: needs to be updated after API Declarative changes
    let steps = generate_steps(&diff, &target, order.domain);

    Ok(steps)
}

struct StateDiff {
    needs_start: bool,
    needs_stop: bool,
    needs_enable: bool,
    needs_disable: bool,
    needs_mask: bool,
    needs_unmask: bool,
}

impl StateDiff {
    pub fn calculate_diff(current: &ServiceState, desired: &ServiceState) -> StateDiff {
        StateDiff {
            // Need to start if: desired=active but current=inactive
            needs_start: desired.active && !current.active,

            // Need to stop if: desired=inactive but current=active
            needs_stop: !desired.active && current.active,

            // Need to enable if: desired=enabled but current=disabled
            needs_enable: desired.enabled && !current.enabled && !current.masked,

            // Need to disable if: desired=disabled but current=enabled
            needs_disable: !desired.enabled && current.enabled,

            // Need to mask if: desired=masked but current=not masked
            needs_mask: desired.masked && !current.masked,

            // Need to unmask if: desired=not masked but current=masked
            needs_unmask: !desired.masked && current.masked,
        }
    }
}

fn generate_steps(diff: &StateDiff, service: &str, domain: Domain) -> Vec<Step> {
    let mut steps = Vec::new();

    // Order matters! Unmask before enable, enable before start...

    if diff.needs_unmask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Mask(false),
            target: service.to_string(),
            description: format!("Unmask {}", service),
        });
    }

    if diff.needs_mask {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Mask(true),
            target: service.to_string(),
            description: format!("Mask {}", service),
        });
    }

    if diff.needs_enable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Enable(true),
            target: service.to_string(),
            description: format!("Enable {}", service),
        });
    }

    if diff.needs_disable {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Enable(false),
            target: service.to_string(),
            description: format!("Disable {}", service),
        });
    }

    if diff.needs_start {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Start(true),
            target: service.to_string(),
            description: format!("Start {}", service),
        });
    }

    if diff.needs_stop {
        steps.push(Step {
            domain: domain.clone(),
            action: Action::Start(false),
            target: service.to_string(),
            description: format!("Stop {}", service),
        });
    }

    steps
}
