use crate::{Order, Plan, module_resolver::ModuleId};
mod services;

pub fn execute(order: &Order, module_id: &ModuleId) -> Result<Vec<String>, String> {
    match module_id {
        ModuleId::Services => services::execute_services(order),
    }
}

pub fn execute_plan(plan: &Plan, module_id: &ModuleId) -> Result<Vec<String>, Vec<String>> {
    match module_id {
        ModuleId::Services => services::execute_services_plan(plan),
    }
}
