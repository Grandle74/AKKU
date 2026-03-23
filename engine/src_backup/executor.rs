// engine/executor.rs
use crate::{Action, Order, action_result_formatter, planner};

pub fn execute_service_order(order: &Order) {
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
                plan.steps
                    .iter()
                    .for_each(|step| execute_service_step(step, order));
            }
        }
    }
}

fn execute_service_step(step: &planner::Step, order: &Order) {
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
