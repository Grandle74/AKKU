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
    Mask,
    Unmask,
    Status,
}

pub enum ActionNoArgs {
    //the "actions" the user wants to perform
    List,
    Help,
    Reset,
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
            ActionNoArgs::Reset => {
                services::reset_service();
            }
        },
        OrderType::Args(more) => match more.action {
            ActionArgs::Status => {
                services::status_service(more.arguments);
            }
            ActionArgs::Start => {
                action_output(more, "starting");
            }
            ActionArgs::Stop => {
                action_output(more, "stopping");
            }
            ActionArgs::Mask => {
                services::mask_service(more.arguments);
            }
            ActionArgs::Unmask => {
                services::unmask_service(more.arguments);
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

/*
//  Output Design

if is_error {
    println!("✗ Service starting failed → {}.service", service[0]);
} else {
    println!("✓ Service starting successed → {}.service", service[0]);
}

for s in 0..vals.len() {
    println!("   → {}", vals[s]);
}
*/

fn action_output(more: OrderArgs, action: &str) {
    let service_name = &more.arguments[0];
    // temporary functionality
    let result = if action == "starting" {
        services::start_service(&more.arguments)
    } else {
        services::stop_service(&more.arguments)
    };

    match result {
        // Every thing goes well here! XD
        Ok(vals) => {
            println!("✓ Service {} successed → {}.service", action, service_name);
            for s in 0..vals.len() {
                println!("   → {}", vals[s]);
            }
        }
        // Here we handle Service Error Case
        Err(vals) => {
            println!("✗ Service {} failed → {}.service", action, service_name);
            for s in 0..vals.len() {
                println!("   → {}", vals[s]);
            }
        }
    }
}
