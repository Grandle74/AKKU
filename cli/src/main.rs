use api::process_intent;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool. Have fun!\n________________________________________________________"
    );
    println!("Enter \"help\" for commands list");

    'com_loop: loop {
        print!("commando(v0.1)~> ");
        io::stdout().flush().unwrap();

        // Read and parse input
        let mut command_in = String::new();
        io::stdin().read_line(&mut command_in).unwrap();

        let command_parts: Vec<String> = command_in
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // Handle empty input
        if command_parts.is_empty() {
            continue;
        }

        // Process command
        match command_parts[0].as_str() {
            "help" => show_help(),
            "exit" | "quit" => {
                println!("Exiting...");
                break 'com_loop;
            }
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().unwrap();
            }

            _ => handle_command(&command_parts),
        }
    }
}

fn show_help() {
    println!("=========== Available Commands ===========");
    println!("help              - Show this help message");
    println!("clear             - Clear the screen");
    println!("exit              - Exit the program");
    println!("=========== Modules Commands =============");
    println!("service           - Manage services");
    println!("network           - Configure network interfaces (unavailable yet)");
}

fn handle_command(parts: &[String]) {
    let domain = &parts[0];

    match parts.len() {
        1 => {
            // Just domain name, no action
            println!("Usage: {} <action> [arguments]", domain);
            println!("See '{} help' for more information.", domain);
        }
        2 => {
            // domain <action>
            match process_intent(domain, &parts[1], None) {
                Ok(_order) => {
                    // API handles everything from here
                }
                Err(err) => println!("Error: {}", err),
            }
        }
        _ => {
            // domain <action> <args...>
            let args = parts[2..].to_vec();
            match process_intent(domain, &parts[1], Some(args)) {
                Ok(_order) => {
                    // API handles everything from here
                }
                Err(err) => println!("Error: {}", err),
            }
        }
    }
}
