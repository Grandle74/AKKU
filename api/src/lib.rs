use engine::{Action, Domain, OrderMore, OrderTwo};

//function must not return a value, but to manage the order and send it to the engl
pub fn two_com_to_ord(command: Vec<String>) -> Result<OrderTwo, String> {
    match command[0].as_str() {
        "service" => {
            let action = service_action_matcher(&command[1])?; // This unwraps or returns error
            Ok(OrderTwo {
                domain: Domain::Services,
                action,
                target: "".to_string(),
            })
        }
        _ => Err("Command not recognized - see \"help\" command for more information.".to_string()),
    }
}

pub fn with_arg_com_to_ord(command: Vec<&str>) {
    match command[0] {
        "service" => {}
        _ => panic!("Command not recognized - see \"help\" command for more information."),
    }
}

fn service_action_matcher(actions: &String) -> Result<Action, String> {
    match actions.as_str() {
        "run" | "start" => Ok(Action::Start),
        "stop" | "kill" => Ok(Action::Stop),
        "remove" | "delete" => Ok(Action::Remove),
        "list" | "show" => Ok(Action::List),
        "enable" | "allow" => Ok(Action::Enable),
        "disable" | "deny" => Ok(Action::Disable),
        "reload" | "restart" => Ok(Action::Reload),
        "help" => Ok(Action::Help),
        _ => Err(
            "Command not recognized - see \"service help\" command for more information."
                .to_string(),
        ),
    }
}

//add a function that sends the converted order to the engine
