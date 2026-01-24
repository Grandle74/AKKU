pub struct OrderNoArgs {
    //idk what kind of type to use here for now
    pub domain: Domain,
    pub action: ActionNoArgs,
}

pub struct OrderArgs {
    //idk what kind of type to use here for now
    pub domain: Domain,
    pub action: ActionArgs,
    pub arguments: Vec<String>,
}

pub enum ActionType {
    Args(ActionArgs),
    NoArgs(ActionNoArgs),
}

pub enum Domain {
    //the "modules" the user wants to interact with
    Services,
}

pub enum ActionArgs {
    Reload,
    Enable,
    Disable,
    Start,
    Stop,
    Remove,
    Status,
}

pub enum ActionNoArgs {
    //the "actions" the user wants to perform
    List,
    Help,
}

pub enum OrderType {
    NoArgs(OrderNoArgs),
    Args(OrderArgs),
}

pub fn service(order: OrderType) {
    match order {
        OrderType::NoArgs(two) => match two.action {
            ActionNoArgs::List => {
                services::list_services();
            }

            ActionNoArgs::Help => {
                services::help_service();
            }
        },
        OrderType::Args(more) => match more.action {
            ActionArgs::Status => {
                //just debuuging the service_status func..
                println!(
                    "service status: {:?}",
                    services::status_service(more.arguments)
                );
            }
            ActionArgs::Start => {
                services::start_service(more.arguments);
            }
            ActionArgs::Stop => {
                services::stop_service(more.arguments);
            }
            ActionArgs::Remove => {
                services::remove_service(more.arguments);
            }
            ActionArgs::Enable => {
                services::enable_service(more.arguments);
            }
            ActionArgs::Disable => {
                services::disable_service(more.arguments);
            }
            ActionArgs::Reload => {
                services::reload_service(more.arguments);
            }
        },
    }
}

/*print!("Enter service name: ");
io::stdout().flush().unwrap();
let mut service_name = String::new();
io::stdin().read_line(&mut service_name).unwrap();
service_name = service_name.trim().to_string();
let status = services::service_status(&service_name);
println!("service status: {}, {}", status[0], status[1]);
*/
