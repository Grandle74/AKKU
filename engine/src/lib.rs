// engine/src/lib.rs
use std::collections::HashMap;
mod action_result_formatter;
mod planner;

#[derive(Debug, Clone)]
pub enum Domain {
    Services,
    // Future: Network, Users, ...
}

#[derive(Debug, Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub desired_properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    // Action — single explicit operation, some is used only by Config Action
    Start,
    Stop,
    Enable,
    Disable,
    Mask,
    Unmask,
    Reload,
    Status,
    // Meta — no target, no state change
    List,
    Help,
    Reset,
    // Declarative — converge to desired state via planner
    Config,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    String(String),
    Number(i64),
}

impl PropertyValue {
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_string(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_number(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

pub fn execute_order(order: &Order) {
    match order.domain {
        Domain::Services => execute_service_order(order),
    }
}

fn execute_service_order(order: &Order) {
    let is_meta = matches!(order.action, Action::List | Action::Help | Action::Reset);

    if is_meta && order.desired_properties.is_empty() {
        // Meta: no target, no properties
        match order.action {
            Action::List => services::list_services(),
            Action::Help => services::help_service(),
            Action::Reset => action_result_formatter::action_output(order, "resetting"),
            _ => {}
        }
    } else if !is_meta && order.desired_properties.is_empty() {
        // Imperative: target only, no properties
        match order.action {
            Action::Status => services::status_service(Some(vec![order.target.clone()])),
            Action::Reload => action_result_formatter::action_output(order, "reloading"),
            _ => {} // Invalid commands are caught by the API before reaching here
        }
    } else {
        // Declarative: execute plan produced by planner
        match planner::create_plan(order.clone()) {
            Err(e) => println!("✗ Planning failed: {}", e),
            Ok(plan) if plan.steps.is_empty() => {
                println!("✓ Already in desired state — no changes needed");
            }
            Ok(plan) => {
                println!("=== Execution Plan ===");
                for (i, step) in plan.steps.iter().enumerate() {
                    println!("{}. {}", i + 1, step.description);
                }
                println!();
                plan.steps.iter().for_each(|step| execute_step(step, order));
            }
        }
    }
}

fn execute_step(step: &planner::Step, order: &Order) {
    let verb = match step.action {
        Action::Start => "starting",
        Action::Stop => "stopping",
        Action::Enable => "enabling",
        Action::Disable => "disabling",
        Action::Mask => "masking",
        Action::Unmask => "unmasking",
        Action::Reload => "reloading",
        _ => return,
    };
    action_result_formatter::action_output(order, verb);
}
