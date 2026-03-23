// engine/src/executor.rs
use crate::{Action, Order, module_resolver::ModuleId};

pub fn execute(order: &Order, module_id: ModuleId) -> Result<(), String> {
    match module_id {
        ModuleId::Services => execute_services(order),
    }
}

fn execute_services(order: &Order) -> Result<(), String> {
    match &order.action {
        Action::Meta(a) => match a.as_str() {
            "list" => {
                services::list_services();
                Ok(())
            }
            "help" => {
                services::help_service();
                Ok(())
            }
            "reset" => {
                println!("resetting...");
                Ok(())
            }
            _ => Err(format!("Unknown meta action '{}'", a)),
        },
        Action::Custom(a) => match a.as_str() {
            "status" => {
                // Convert target to Option<Vec<String>> for status_service
                // This is a temporary conversion until status_service accepts Option<Vec<String>>
                let order_con: Option<Vec<String>> = order.target.clone().map(|s| vec![s]);
                services::status_service(order_con);
                Ok(())
            }
            _ => Err(format!("Unknown action '{}'", a)),
        },
        Action::Config => {
            todo!("Config execution is missing :\\");
        }
    }
}
