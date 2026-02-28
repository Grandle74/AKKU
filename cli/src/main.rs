// cli/src/main.rs
use api::{PropertyValue, process_bi_intent, process_tri_intent};
use std::collections::HashMap;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool.\n{}",
        "─".repeat(56)
    );
    println!("Enter \"help\" for commands list\n");

    'repl: loop {
        print!("commando(v0.1)~> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let parts: Vec<String> = input.trim().split_whitespace().map(String::from).collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].as_str() {
            "help" => show_help(),
            "exit" | "quit" => {
                println!("Exiting...");
                break 'repl;
            }
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().unwrap();
            }
            _ => handle_intent(&parts),
        }
    }
}

fn show_help() {
    println!("\n=== Available Commands ===");
    println!("  help              - Show this help message");
    println!("  clear             - Clear the screen");
    println!("  exit              - Exit the program");
    println!("\n=== Module Commands ===");
    println!("  Imperative style (single action):");
    println!("    service <action> [name]");
    println!("    Examples:");
    println!("      service list");
    println!("      service start nginx");
    println!("      service status nginx");
    println!("\n  Declarative style (desired state):");
    println!("    service <name> change <property>=<value> ...");
    println!("    Properties: running, enabled, masked");
    println!("    Examples:");
    println!("      service nginx change running=true enabled=true");
    println!("      service nginx change masked=false\n");
}

fn handle_intent(parts: &[String]) {
    let domain = &parts[0];

    match parts.len() {
        // domain (no action given)
        1 => println!("See '{} help' for more information.", domain),

        // domain <action>  →  list, help, reset, ...
        2 => {
            if let Err(e) = process_bi_intent(domain, &parts[1]) {
                println!("{}", e);
            }
        }

        // domain <action> <target>  →  status nginx, start nginx, ...
        3 => {
            if let Err(e) =
                process_tri_intent(domain, parts[1].clone(), parts[2].clone(), HashMap::new())
            {
                println!("{}", e);
            }
        }

        // domain <target> change <property>=<value> ...  →  declarative
        _ => handle_declarative(domain, parts),
    }
}

fn handle_declarative(domain: &str, parts: &[String]) {
    let action = parts[1].to_string();
    let target = parts[2].to_string();
    let mut properties = HashMap::new();

    if action != "change" && action != "config" {
        println!("✗ Error: Invalid command — check '{} help'", domain);
        return;
    }

    for token in &parts[3..] {
        match token.split_once('=') {
            // valid key=value
            Some((key, value)) if !key.is_empty() && !value.is_empty() => {
                let parsed = match value {
                    "true" | "yes" | "1" => PropertyValue::Bool(true),
                    "false" | "no" | "0" => PropertyValue::Bool(false),
                    _ => value
                        .parse::<i64>()
                        .map(PropertyValue::Number)
                        .unwrap_or_else(|_| PropertyValue::String(value.to_string())),
                };
                // This part prevents from having the same property/key multiple times
                if properties.contains_key(key) {
                    println!(
                        "✗ Error: Duplicated property '{}' — check '{} help'",
                        key, domain
                    );
                    return;
                }
                properties.insert(key.to_string(), parsed);
            }
            // =value or = (no key)
            Some((key, _)) if key.is_empty() => {
                println!("✗ Error: Invalid property — check '{} help'", domain);
                return;
            }
            // key= (no value)
            Some(_) => {
                println!("✗ Error: Invalid property value — check '{} help'", domain);
                return;
            }
            // no '=' at all — prop written without assignment
            None => {
                println!(
                    "✗ Error: Property must be in key=value format — check '{} help'",
                    domain
                );
                return;
            }
        }
    }

    if properties.is_empty() {
        println!("✗ Error: No properties provided — check '{} help'", domain);
        return;
    }

    if let Err(e) = process_tri_intent(domain, action, target, properties) {
        println!("✗ Error: {}", e);
    }
}
