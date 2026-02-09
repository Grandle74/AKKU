// Future API structure design - Declarative System
pub struct Order {
    pub domain: Domain,
    pub desired_state: StateDefinition, // ← What user wants
    pub current_state: Option<StateDefinition>, // ← What exists now
}

pub struct StateDefinition {
    pub services: Vec<ServiceState>,
    // pub networks: Vec<NetworkState>,
    // pub users: Vec<UserState>,
}

pub struct ServiceState {
    pub name: String,
    pub should_be_running: bool,
    pub should_be_enabled: bool,
    pub should_be_masked: bool,
}
