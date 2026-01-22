use api::command_to_order;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");

    println!("This is a prototype of YaST3, a system configuration tool.\n");

    println!("enter \"help\" for commands list");
    'com_loop: loop {
        print!("commando(v0.1)~> ");
        io::stdout().flush().unwrap();
        //input method:
        let mut command_in = String::new();
        io::stdin().read_line(&mut command_in).unwrap();
        let command = command_in.split_whitespace().collect::<Vec<&str>>();
        //check if command is valid
        if command.len() == 3 {
            //let command: Vec<String> = command.iter().map(|s| s.to_string()).collect();
            command_to_order(command);
        } else {
            println!(
                "Invalid command - see \"{} help\" command for more information.",
                command[0]
            );
        }
        /*
                match input.trim() {
                    "help" => {
                        println!("Available commands:");
                        println!("network - Configure network settings (unavailable yet)");
                        println!("user - Manage user accounts (unavailable yet)");
                        println!("package - Install software packages (unavailable yet)");
                        println!("service - Configure system services");
                        println!("exit - Exit");
                    }
                    "networks" => println!("Network settings configuration is not available yet."),
                    "users" => println!("User account management is not available yet."),
                    "packages" => println!("Software package installation is not available yet."),
                    "service" => {
                        println!("Starting System services configuration...");
                        services_cli::service_cli();
                    }
                    _ => {
                        println!("Exiting YaST3 (prototype)...");
                        break 'com_loop;
                    }
                }
        */
    }
}
