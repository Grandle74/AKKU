mod action_result_formatter;

pub enum Domain {
    Services,
}

pub struct Order {
    pub domain: Domain,
    pub action: Action,
    pub arguments: Option<Vec<String>>,
}

pub enum Action {
    Reload,
    Enable,
    Disable,
    Start,
    Stop,
    Mask,
    Unmask,
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
    match order.action {
        Action::List => services::list_services(),
        Action::Help => services::help_service(),
        Action::Reset => services::reset_service(),
        Action::Status => services::status_service(order.arguments),
        Action::Start => action_result_formatter::action_output(order, "starting"),
        Action::Stop => action_result_formatter::action_output(order, "stopping"),
        Action::Mask => action_result_formatter::action_output(order, "masking"),
        Action::Unmask => action_result_formatter::action_output(order, "unmasking"),
        Action::Enable => services::enable_service(order.arguments),
        Action::Disable => services::disable_service(order.arguments),
        Action::Reload => services::reload_service(order.arguments),
    }
}
