use std::io::{self, Write};

pub struct OrderTwo {
    //idk what kind of type to use here for now
    pub domain: Domain,
    pub action: Action,
    pub target: String,
}
pub struct OrderMore {
    //idk what kind of type to use here for now
    pub domain: Domain,
    pub action: Action,
    pub target: Vec<String>,
}

pub enum Domain {
    //the "modules" the user wants to interact with
    Services,
}

pub enum Action {
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

pub fn service(order: OrderTwo) {
    print!("Enter service name: ");
    io::stdout().flush().unwrap();
    let mut service_name = String::new();
    io::stdin().read_line(&mut service_name).unwrap();
    service_name = service_name.trim().to_string();
    let status = services::service_status(&service_name);
    println!("service status: {}, {}", status[0], status[1]);
}
