use api::{PropertyValue, approve_intent, process_bi_intent, process_tri_intent};
use std::collections::HashMap;
use std::io::{self, Write};

// main(), show_help(), print_lines() — completely unchanged
fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool.\n{}",
        "─".repeat(56)
    );
    println!("Enter \"help\" for commands list\n");

    'replay: loop {
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
                break 'replay;
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
    println!("    service change <name> <property>=<value> ...");
    println!("    Properties: running, enabled, masked");
    println!("    Examples:");
    println!("      service change nginx running=true enabled=true");
    println!("      service change nginx masked=false\n");
}

fn handle_intent(parts: &[String]) {
    let domain = &parts[0];

    match parts.len() {
        1 => println!("See '{} help' for more information.", domain),

        // Bi-intent: unchanged, still returns Vec<String> directly
        2 => match process_bi_intent(domain, &parts[1]) {
            Ok(output) => print_lines(output),
            Err(errors) => {
                print!("✗ Error: ");
                print_lines(errors)
            }
        },

        // Tri-intent: now returns IntentResult — check for pending plan
        3 => match process_tri_intent(domain, parts[1].clone(), parts[2].clone(), HashMap::new()) {
            Ok(result) => {
                print_lines(result.output);
                // Custom actions (status, start, ...) never produce a plan.
                // pending_plan is always None here — this branch is a no-op for now.
                handle_pending_plan(result.pending_plan);
            }
            Err(errors) => {
                print!("✗ Error: ");
                print_lines(errors)
            }
        },

        // Declarative: domain <target> change <key>=<value> ...
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

    // Property parsing — unchanged
    for token in &parts[3..] {
        match token.split_once('=') {
            Some((key, value)) if !key.is_empty() && !value.is_empty() => {
                let parsed = match value {
                    "true" | "yes" | "1" => PropertyValue::Bool(true),
                    "false" | "no" | "0" => PropertyValue::Bool(false),
                    _ => value
                        .parse::<i64>()
                        .map(PropertyValue::Number)
                        .unwrap_or_else(|_| PropertyValue::String(value.to_string())),
                };
                if properties.contains_key(key) {
                    println!(
                        "✗ Error: Duplicated property '{}' — check '{} help'",
                        key, domain
                    );
                    return;
                }
                properties.insert(key.to_string(), parsed);
            }
            Some((key, _)) if key.is_empty() => {
                println!("✗ Error: Invalid property — check '{} help'", domain);
                return;
            }
            Some(_) => {
                println!("✗ Error: Invalid property value — check '{} help'", domain);
                return;
            }
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

    match process_tri_intent(domain, action, target, properties) {
        Ok(result) => {
            print_lines(result.output);
            // Config actions always return a pending plan — ask the user.
            handle_pending_plan(result.pending_plan);
        }
        Err(errors) => {
            print!("✗ Error: ");
            print_lines(errors);
        }
    }
}

/// Handles the approval flow when a Config action produced a plan.
/// Called after showing the plan output — asks the user yes/no, then acts.
fn handle_pending_plan(pending_plan: Option<api::Plan>) {
    let Some(plan) = pending_plan else { return };

    // Ask the user for approval
    print!("\nApply this plan? [y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");

    match approve_intent(plan, approved) {
        Ok(output) => print_lines(output),
        Err(errors) => {
            print!("✗ Error: ");
            print_lines(errors);
        }
    }
}

// Printing results helper
fn print_lines<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) {
    for item in items {
        println!("{}", item);
    }
}
