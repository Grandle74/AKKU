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

pub fn service(order: Order) {
    match order.action {
        Action::List => {
            services::list_services();
        }
        Action::Help => {
            services::help_service();
        }
        Action::Reset => {
            services::reset_service();
        }
        Action::Status => {
            services::status_service(order.arguments);
        }
        Action::Start => {
            action_result_formatter::action_output(order, "starting");
        }
        Action::Stop => {
            action_result_formatter::action_output(order, "stopping");
        }
        Action::Mask => {
            services::mask_service(order.arguments);
        }
        Action::Unmask => {
            services::unmask_service(order.arguments);
        }
        Action::Enable => {
            services::enable_service(order.arguments);
        }
        Action::Disable => {
            services::disable_service(order.arguments);
        }
        Action::Reload => {
            services::reload_service(order.arguments);
        }
    }
}
