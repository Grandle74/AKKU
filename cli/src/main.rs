use api::two_com_to_ord;
use engine::service;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");

    println!("This is a prototype of YaST3, a system configuration tool. Have fun!\n");

    println!("enter \"help\" for commands list");
    'com_loop: loop {
        print!("commando(v0.1)~> ");
        io::stdout().flush().unwrap();
        //input method:
        let mut command_in = String::new();
        io::stdin().read_line(&mut command_in).unwrap();
        let command_splitted = command_in
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        //check if command is valid
        match command_splitted.len() {
            0 => {}
            1 => match command_splitted[0].as_str() {
                "help" => {}
                "exit" => {
                    println!("Exiting...");
                    break 'com_loop;
                }
                "service" => {
                    println!("see \"service help\" command for more information.");
                }
                _ => println!("Invalid command - see \"help\" command for more information."),
            },
            2 => match two_com_to_ord(command_splitted) {
                Ok(order) => {
                    service(order);
                }
                Err(err) => {
                    println!("Error: {}", err);
                }
            },
            _ => {
                let command: (String, String, Vec<String>) = {
                    let command = command_splitted[0].to_string();
                    let subcommand = command_splitted[1].to_string();
                    let args = command_splitted[2..].to_vec();
                    (command, subcommand, args)
                };
                println!("this is a 3+ words command.");
            }
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
