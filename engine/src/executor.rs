use crate::{Action, Order, Plan, module_resolver::ModuleId};

pub fn execute(order: &Order, module_id: &ModuleId) -> Result<Vec<String>, String> {
    match module_id {
        ModuleId::Services => execute_services(order),
    }
}

pub fn execute_plan(plan: &Plan, module_id: &ModuleId) -> Result<Vec<String>, Vec<String>> {
    match module_id {
        ModuleId::Services => execute_services_plan(plan),
    }
}

fn execute_services(order: &Order) -> Result<Vec<String>, String> {
    match &order.action {
        Action::Meta(a) => match a.as_str() {
            "list" => {
                // todo: return structured data instead of strings (needed for GUI/TUI frontends)
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
                let target = order.target.clone().map(|s| vec![s]);

                services::status_service(target)
            }
            _ => Err(format!("Unknown action '{}'", a)),
        },
        Action::Config => unreachable!("Config should use execute_plan"),
    }
}

fn execute_services_plan(plan: &Plan) -> Result<Vec<String>, Vec<String>> {
    let mut output = vec![];
    for step in &plan.steps {
        match &step.action {
            Action::Custom(a) => {
                let target = &Some(vec![step.target.clone()]);
                let result = match a.as_str() {
                    "start" => services::start_service(target),
                    "stop" => services::stop_service(target),
                    "enable" => services::enable_service(target),
                    "disable" => services::disable_service(target),
                    "mask" => services::mask_service(target),
                    "unmask" => services::unmask_service(target),
                    _ => return Err(vec![format!("Unknown step '{}'", a)]),
                };
                match result {
                    Ok(mut out) => output.append(&mut out),
                    Err(e) => return Err(e), // fail fast on first error
                }
            }
            _ => return Err(vec!["Non-custom action in plan".to_string()]),
        }
    }
    Ok(output)
}
