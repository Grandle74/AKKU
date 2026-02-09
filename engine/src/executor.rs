// engine/executor.rs
pub fn run_plan(plan: &ExecutionPlan) -> Result<(), String> {
    for step in &plan.steps {
        match step.domain {
            Domain::Services => {
                execute_service_step(step)?;
            } // Future: Domain::Network => execute_network_step(step)?
        }
    }
    Ok(())
}

fn execute_service_step(step: &Step) -> Result<(), String> {
    // Call your existing service functions
    let args = Some(vec![step.target.clone()]);

    match step.action {
        Action::Start => services::start_service(&args)
            .map(|_| ())
            .map_err(|e| e.join(", ")),
        Action::Stop => services::stop_service(&args)
            .map(|_| ())
            .map_err(|e| e.join(", ")),
        // ... etc
    }
}
