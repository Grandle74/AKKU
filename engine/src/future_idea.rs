// Future engine (planning & orchestration)
fn execute_service_order(order: Order) {
    // Plan a sequence of actions
    let plan = match order.action {
        Action::Start => {
            vec![
                check_if_service_exists(),
                check_dependencies(),
                unmask_if_masked(),
                enable_if_disabled(),
                actually_start_service(),
                verify_started(),
                update_status_cache(),
            ]
        }
        Action::Stop => {
            vec![
                check_if_running(),
                check_dependent_services(),
                stop_dependent_services_first(),
                actually_stop_service(),
                verify_stopped(),
            ]
        } // Complex orchestration...
    };

    // Execute the plan
    for step in plan {
        step.execute()?;
    }
}
