mod action_result_formatter;
mod planner;

#[derive(Debug, Clone)]
pub enum Domain {
    Services,
}
#[derive(Clone)]
pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub arguments: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum Action {
    // boolean is used in the way: Start(true) = Start / Start(false) = Stop
    Reload,
    Enable(bool),
    Start(bool),
    Mask(bool),
    Status,
    List,
    Help,
    Reset,
}

/// Main engine entry point - the ONLY public function
pub fn execute_order(order: Order) {
    match order.domain {
        Domain::Services => execute_service_order(order),
        // Future:
        // Domain::Network => execute_network_order(order),
        // Domain::User => execute_user_order(order),
    }
}

/// Internal: Handle service domain orders
fn execute_service_order(order: Order) {
    // DBG
    println!("Plan is:\n{:?}", planner::create_plan(order.clone()));

    match order.action {
        Action::List => services::list_services(),
        Action::Help => services::help_service(),
        Action::Status => services::status_service(order.arguments),
        Action::Reset => action_result_formatter::action_output(order, "resetting"),
        Action::Start(true) => action_result_formatter::action_output(order, "starting"),
        Action::Start(false) => action_result_formatter::action_output(order, "stopping"),
        Action::Mask(true) => action_result_formatter::action_output(order, "masking"),
        Action::Mask(false) => action_result_formatter::action_output(order, "unmasking"),
        Action::Enable(true) => action_result_formatter::action_output(order, "enabling"),
        Action::Enable(false) => action_result_formatter::action_output(order, "disabling"),
        Action::Reload => action_result_formatter::action_output(order, "reloading"),
    }
}
