// engine/src/executor/services.rs
//
// Systemd-specific dispatch between the engine executor and the services module.
//
// Temporary stand-in — when the Module Bundle design is introduced, action
// dispatch moves into each bundle's engine-side handlers.
// This file will be removed at that point.
// See ModulesManager.md.
//
// This is the only file in the engine that imports the services crate.

use crate::{Action, Order, Plan};

/// Routes an imperative Order to the correct services function.
///
/// Meta actions require no target. Custom actions require a target in the Order.
/// Config must never reach here — it is caught as unreachable to surface
/// routing bugs rather than silently misbehaving.
pub fn execute_services(order: &Order) -> Result<Vec<String>, String> {
    match &order.action {
        Action::Meta(a) => match a.as_str() {
            "list" => {
                // TODO: Return structured ServiceEntry data instead of formatted strings.
                // Formatted strings work for the CLI but prevent future GUI/TUI frontends
                // from applying their own layout. Deferred until a second frontend exists.
                let entries = services::list_services()?;
                Ok(entries
                    .iter()
                    .map(|e| {
                        format!(
                            "{:<40} {:<10} {:<10} {}",
                            e.name, e.load_state, e.active, e.description
                        )
                    })
                    .collect())
            }
            "help" => Ok(services::help_service()),
            "clean" => services::reset_service(),
            _ => Err(format!("Unknown meta action '{}'", a)),
        },

        Action::Custom(a) => match a.as_str() {
            "status" => {
                let target = order.target.as_deref().ok_or("No target provided")?;
                services::status_service(target)
            }
            "reload" => {
                let target = order.target.as_deref().ok_or("No target provided")?;
                services::reload_service(target)
            }
            _ => Err(format!("Unknown action '{}'", a)),
        },

        Action::Config => {
            // Config actions are planned and approved before execution.
            // Reaching this branch means the engine has a routing bug.
            unreachable!("Config actions must go through execute_plan, not execute_normal")
        }
    }
}

/// Executes a pre-approved Plan by running its Steps in order.
///
/// Fails fast on the first error — subsequent steps are not attempted.
/// Steps are ordered with dependencies (unmask → enable → start), so a
/// failed early step makes later steps either meaningless or harmful.
pub fn execute_services_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    let mut output = vec![];

    for (index, step) in plan.steps.iter().enumerate() {
        match &step.action {
            Action::Custom(action) => {
                let result = match action.as_str() {
                    "start" => services::start_service(&step.target),
                    "stop" => services::stop_service(&step.target),
                    "enable" => services::enable_service(&step.target),
                    "disable" => services::disable_service(&step.target),
                    "mask" => services::mask_service(&step.target),
                    "unmask" => services::unmask_service(&step.target),
                    "reset" => services::reset_failed_service(&step.target),
                    _ => return Err(vec![format!("Unknown step action '{}'", action)]),
                };

                match result {
                    Ok(mut lines) => {
                        let _ = crate::plan_store::update_step_status(
                            &plan.id,
                            index,
                            "completed",
                            &lines,
                        );
                        output.append(&mut lines);
                    }
                    Err(e) => {
                        let error_lines: Vec<String> = e.lines().map(String::from).collect();
                        let _ = crate::plan_store::update_step_status(
                            &plan.id,
                            index,
                            "failed",
                            &error_lines,
                        );
                        return Err(vec![e]);
                    }
                }
            }

            _ => {
                // All Steps produced by to_steps() are Custom. Any other variant
                // here means a Step was constructed incorrectly — surface it as a bug.
                return Err(vec![
                    "Non-Custom action found inside a Plan — this is a bug.".to_string(),
                ]);
            }
        }
    }

    Ok(output)
}
