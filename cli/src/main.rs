// cli/src/main.rs
use api::{PropertyValue, process_bi_command, process_tri_command};
use std::collections::HashMap;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool. Have fun!\n________________________________________________________"
    );
    println!("Enter \"help\" for commands list\n");

    'com_loop: loop {
        print!("commando(v0.1)~> ");
        io::stdout().flush().unwrap();

        let mut command_in = String::new();
        io::stdin().read_line(&mut command_in).unwrap();

        let command_parts: Vec<String> = command_in
            .trim()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if command_parts.is_empty() {
            continue;
        }

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
    println!("\n=== Available Commands ===");
    println!("  help              - Show this help message");
    println!("  clear             - Clear the screen");
    println!("  exit              - Exit the program");
    println!("\n=== Module Commands ===");
    println!("  Imperative style (execute single action):");
    println!("    service <action> [name]");
    println!("    Examples:");
    println!("      service list");
    println!("      service start nginx");
    println!("      service status nginx");
    println!("\n  Declarative style (specify desired state):");
    println!("    service <name> <property>=<value> ...");
    println!("    Properties: running, enabled, masked");
    println!("    Examples:");
    println!("      service nginx running=true enabled=true");
    println!("      service nginx masked=false");
    println!("      service nginx running=true enabled=true masked=false\n");
}

fn handle_command(parts: &[String]) {
    let domain = &parts[0];

    match parts.len() {
        1 => {
            // It doesn't tell you if the Command exists - not its job!
            println!("See '{} help' for more information.", domain);
        }
        2 => {
            let process = process_bi_command(domain, parts[1].as_str());
            if let Some(e) = process.err() {
                println!("{}", e);
            }
        }
        _ => {
            handle_declarative(domain, parts);
        }
    }
}

fn handle_declarative(domain: &str, parts: &[String]) {
    let target = parts[1].to_string();
    let mut properties = HashMap::new();

    // Parse key=value pairs
    for prop_str in &parts[2..] {
        if let Some((key, value)) = prop_str.split_once('=') {
            let parsed_value = match value {
                "true" | "yes" | "1" => PropertyValue::Bool(true),
                "false" | "no" | "0" => PropertyValue::Bool(false),
                _ => {
                    // Try to parse as number, if no then as a string
                    if let Ok(num) = value.parse::<i64>() {
                        PropertyValue::Number(num)
                    } else {
                        PropertyValue::String(value.to_string())
                    }
                }
            };
            properties.insert(key.to_string(), parsed_value);
        }
    }

    if properties.is_empty() {
        println!("✗ Error: No properties specified");
        return;
    }

    match process_tri_command(domain, target, properties) {
        Ok(_) => {}
        Err(err) => println!("✗ Error: {}", err),
    }
}
