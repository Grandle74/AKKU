// engine/lib.rs
pub mod executor;
pub mod planner;
pub mod rollback;

pub struct ExecutionPlan {
    pub steps: Vec<Step>,
    pub rollback_steps: Vec<Step>,
}

pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub description: String, // Human-readable: "Start nginx service"
}

pub fn execute_order(order: Order) {
    // PHASE 1: Create execution plan
    let plan = planner::create_plan(order);

    // PHASE 2: Dry run (show what will happen)
    if DRY_RUN_MODE {
        // You can add this later
        planner::show_plan(&plan);
        return;
    }

    // PHASE 3: Execute plan
    let result = executor::run_plan(&plan);

    // PHASE 4: Rollback if needed
    if result.is_err() {
        rollback::execute(&plan.rollback_steps);
    }
}
