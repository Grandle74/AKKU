// engine/src/executor/services.rs
use crate::{Action, Order, Plan};

pub fn execute_services(order: &Order) -> Result<Vec<String>, String> {
    match &order.action {
        Action::Meta(a) => match a.as_str() {
            "list" => {
                // TODO: Return structured data instead of strings (needed for future GUI/TUI frontends).
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
            "reset" => Ok(services::reset_service()?),
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
            unreachable!("Config actions must go through execute_plan, not execute_normal")
        }
    }
}

pub fn execute_services_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    let mut output = vec![];

    for step in &plan.steps {
        match &step.action {
            Action::Custom(action) => {
                let result = match action.as_str() {
                    "start" => services::start_service(&step.target),
                    "stop" => services::stop_service(&step.target),
                    "enable" => services::enable_service(&step.target),
                    "disable" => services::disable_service(&step.target),
                    "mask" => services::mask_service(&step.target),
                    "unmask" => services::unmask_service(&step.target),
                    _ => return Err(vec![format!("Unknown step action '{}'", action)]),
                };
                match result {
                    Ok(mut lines) => output.append(&mut lines),
                    Err(e) => return Err(e), // Fail fast on first error.
                }
            }
            _ => {
                return Err(vec![
                    "Non-Custom action found inside a Plan — this is a bug.".to_string(),
                ]);
            }
        }
    }

    Ok(output)
}
