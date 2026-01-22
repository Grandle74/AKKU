use engine::service;

pub struct Order {
    //idk what kind of type to use here for now
    domain: Domain,
    action: Action,
    target: String,
}

enum Domain {
    //the "modules" the user wants to interact with
    Services,
    Help,
}

enum Action {
    //the "actions" the user wants to perform
    Start,
    Stop,
    Remove,
    List,
    Enable,
    Disable,
    Reload,
    Help,
}

pub fn command_to_order(command: Vec<&str>) -> Order {
    //this mathes the first word of the command => the domain
    match command[0] {
        "service" => Order {
            domain: Domain::Services,
            action: service_action_matcher(command[1]),
            target: command[2].to_string(),
        },
        "help" => {
            println!("Available commands:");
            println!("network - Configure network settings (unavailable yet)");
            println!("user - Manage user accounts (unavailable yet)");
            println!("package - Install software packages (unavailable yet)");
            println!("service - Configure system services");
            println!("exit - Exit");
            return Order {
                domain: Domain::Help,
                action: Action::Help,
                target: String::new(),
            };
        }
        _ => panic!("Command not recognized - see \"help\" command for more information."),
    }
}

fn service_action_matcher(actions: &str) -> Action {
    match actions {
        "run" | "start" => Action::Start,
        "stop" | "kill" => Action::Stop,
        "remove" | "delete" => Action::Remove,
        "list" | "show" => Action::List,
        "enable" | "allow" => Action::Enable,
        "disable" | "deny" => Action::Disable,
        "reload" | "restart" => Action::Reload,
        _ => panic!("Command not recognized - see \"service help\" command for more information."),
    }
}
