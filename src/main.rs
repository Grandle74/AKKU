use std::io::{self, Write};
mod api;

fn main() {
    println!("Welcome to YaST3 (prototype)!");

    println!("This is a prototype of YaST3, a system configuration tool.");

    println!("Please select an option:");
    println!("1. Configure network settings (unavailable yet)");
    println!("2. Manage user accounts (unavailable yet)");
    println!("3. Install software packages (unavailable yet)");
    println!("4. Configure system services");
    println!("Or. press any key to Exit");
    print!("commando(v0.1)~> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    match input.trim() {
        "1" => println!("Network settings configuration is not available yet."),
        "2" => println!("User account management is not available yet."),
        "3" => println!("Software package installation is not available yet."),
        "4" => {
            println!("Starting System services configuration...");
            api::service();
        }
        _ => println!("Exiting YaST3 (prototype)..."),
    }
}
