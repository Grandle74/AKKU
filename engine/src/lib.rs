// engine/src/lib.rs
use std::collections::HashMap;

mod action_result_formatter;
mod planner;

#[derive(Debug, Clone)]
pub enum Domain {
    Services,
    // Future: Network, Users, etc.
}

#[derive(Clone, Debug)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub desired_properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Start,
    Stop,
    Enable,
    Disable,
    Mask,
    Unmask,
    Reload,
    Status,
    List,
    Help,
    Reset,
    Config,
}

pub fn execute_order(order: &Order) {
    match order.domain {
        Domain::Services => execute_service_order(order),
    }
}

// This needs to be added: Function returns Result
fn execute_service_order(order: &Order) {
    // Check if meta action (no state changes)
    // FIXED: Check both target AND empty properties
    let is_meta = order.action == Action::List
        || order.action == Action::Help
        || order.action == Action::Reset;

    if is_meta && order.desired_properties.is_empty() {
        match order.action.clone() {
            Action::List => services::list_services(),
            Action::Help => services::help_service(),
            Action::Reset => action_result_formatter::action_output(order, "resetting"),
            _ => {
                // This is "status" - pass the target as argument
                // services::status_service(Some(vec![order.target.clone()]))
            }
        }
        return;
    } else if !is_meta && order.desired_properties.is_empty() {
        match order.action.clone() {
            Action::Status => services::status_service(Some(vec![order.target.clone()])),
            Action::Reload => action_result_formatter::action_output(order, "reloading"),
            _ => {
                // Supposed to return error of invalid command
                return;
            }
        }
        return;
    } else {
        // Create and execute plan
        match planner::create_plan(order.clone()) {
            Ok(plan) => {
                if plan.steps.is_empty() {
                    println!("✓ No changes needed - service already in desired state");
                    return;
                }

                println!("=== Execution Plan ===");
                for (i, step) in plan.steps.iter().enumerate() {
                    println!("{}. {}", i + 1, step.description);
                }
                println!();

                // Execute each step
                for step in &plan.steps {
                    execute_step(step, order);
                }
            }
            Err(e) => println!("✗ Planning failed: {}", e),
        }
    }
}

fn execute_step(step: &planner::Step, order: &Order) {
    match step.action {
        Action::Start => action_result_formatter::action_output(order, "starting"),
        Action::Stop => action_result_formatter::action_output(order, "stopping"),
        Action::Enable => action_result_formatter::action_output(order, "enabling"),
        Action::Disable => action_result_formatter::action_output(order, "disabling"),
        Action::Mask => action_result_formatter::action_output(order, "masking"),
        Action::Unmask => action_result_formatter::action_output(order, "unmasking"),
        Action::Reload => action_result_formatter::action_output(order, "reloading"),
        _ => {}
    }
}
