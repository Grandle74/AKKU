use engine::{ActionArgs, ActionNoArgs, Domain, OrderArgs, OrderNoArgs, OrderType};

//function must not return a value, but to manage the order and send it to the engl
pub fn noargs_com_to_ord(command: Vec<String>) -> Result<OrderType, String> {
    match command[0].as_str() {
        "service" => {
            let action = service_no_args_matcher(&command[1])?; // This unwraps or returns error
            Ok(OrderType::NoArgs(OrderNoArgs {
                domain: Domain::Services,
                action,
            }))
        }
        _ => Err("Command not recognized - see \"help\" command for more information.".to_string()),
    }
}

pub fn com_to_ord(command: (String, String, Vec<String>)) -> Result<OrderType, String> {
    match command.0.as_str() {
        "service" => {
            let action = service_action_matcher(&command.1)?; // This unwraps or returns error
            Ok(OrderType::Args(OrderArgs {
                domain: Domain::Services,
                action,
                arguments: command.2,
            }))
        }
        _ => Err("Command not recognized - see \"help\" command for more information.".to_string()),
    }
}

fn service_no_args_matcher(actions: &String) -> Result<ActionNoArgs, String> {
    match actions.as_str() {
        "list" | "show" => Ok(ActionNoArgs::List),
        "reset" => Ok(ActionNoArgs::Reset),
        "help" => Ok(ActionNoArgs::Help),
        _ => Err(
            "Command not recognized - see \"service help\" command for more information."
                .to_string(),
        ),
    }
}

fn service_action_matcher(actions: &String) -> Result<ActionArgs, String> {
    match actions.as_str() {
        "status" => Ok(ActionArgs::Status),
        "run" | "start" => Ok(ActionArgs::Start),
        "stop" | "kill" => Ok(ActionArgs::Stop),
        "remove" | "delete" => Ok(ActionArgs::Remove),
        "enable" | "allow" => Ok(ActionArgs::Enable),
        "disable" | "deny" => Ok(ActionArgs::Disable),
        "reload" | "restart" => Ok(ActionArgs::Reload),
        _ => Err(
            "Command not recognized - see \"service help\" command for more information."
                .to_string(),
        ),
    }
}

//add a function that sends the converted order to the engine
