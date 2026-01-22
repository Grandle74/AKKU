use services::service_status;
use std::io::{self, Write};

pub fn service() {
    print!("Enter service name: ");
    io::stdout().flush().unwrap();
    let mut service_name = String::new();
    io::stdin().read_line(&mut service_name).unwrap();
    service_name = service_name.trim().to_string();
    let status = services::service_status(&service_name);
    println!("service status: {}, {}", status[0], status[1]);
}
