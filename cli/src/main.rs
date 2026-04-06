// cli/src/main.rs
use api::{PropertyValue, RunMode, approve_intent, process_bi_intent, process_tri_intent};
use rustyline::error::ReadlineError;
use std::collections::HashMap;
use std::io::{self, Write};

fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool.\n{}",
        "─".repeat(56)
    );
    println!("Enter \"help\" for commands list\n");

    let mut rl = rustyline::DefaultEditor::new().expect("Failed to initialize input editor");

    'repl: loop {
        match rl.readline("commando(v0.1)~> ") {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&trimmed);

                let (parts, mode) = parse_flags(&trimmed);
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
                    _ => handle_intent(&parts, mode),
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("Exiting...");
                break 'repl;
            }
            Err(e) => {
                eprintln!("Input error: {e}");
                break 'repl;
            }
        }
    }
}

/// Strips `--dry-run` and `--force` flags from the token list.
/// Returns the clean parts and the resolved RunMode.
/// `--force` takes precedence: if both are present, Force wins.
fn parse_flags(input: &str) -> (Vec<String>, RunMode) {
    let mut dry_run = false;
    let mut force = false;
    let parts: Vec<String> = input
        .split_whitespace()
        .filter(|t| {
            if *t == "--dry-run" {
                dry_run = true;
                false
            } else if *t == "--force" {
                force = true;
                false
            } else {
                true
            }
        })
        .map(String::from)
        .collect();

    let mode = if force {
        RunMode::Force
    } else if dry_run {
        RunMode::DryRun
    } else {
        RunMode::Normal
    };

    (parts, mode)
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
    println!("      service status nginx");
    println!("\n  Declarative style (desired state):");
    println!("    service change <name> <property>=<value> ...");
    println!("    Properties: running, enabled, masked");
    println!("    Examples:");
    println!("      service change nginx running=true enabled=true");
    println!("      service change nginx masked=false\n");
}

fn handle_intent(parts: &[String], mode: RunMode) {
    let domain = &parts[0];

    match parts.len() {
        1 => println!("See '{} help' for more information.", domain),

        2 => {
            let action = &parts[1];
            let result = process_bi_intent(domain, action);
            print_result(action, result);
        }

        3 => {
            let action = &parts[1];
            match process_tri_intent(
                domain,
                action.clone(),
                parts[2].clone(),
                HashMap::new(),
                &mode,
            ) {
                Ok(r) if r.pending_plan.is_some() => {
                    print_lines(r.output);
                    handle_pending_plan(r.pending_plan);
                }
                Ok(r) => print_result(action, Ok(r.output)),
                Err(errors) => print_result(action, Err(errors)),
            }
        }

        _ => handle_declarative(domain, parts, mode),
    }
}

fn handle_declarative(domain: &str, parts: &[String], mode: RunMode) {
    let action = parts[1].to_string();
    let target = parts[2].to_string();
    let mut properties = HashMap::new();

    if action != "change" && action != "config" && action != "cfg" {
        println!("✗ Error: Invalid command — check '{} help'", domain);
        return;
    }

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

    match process_tri_intent(domain, action.clone(), target, properties, &mode) {
        Ok(r) if r.pending_plan.is_some() => {
            print_lines(r.output);
            handle_pending_plan(r.pending_plan);
        }
        Ok(r) => print_result(&action, Ok(r.output)),
        Err(errors) => print_result(&action, Err(errors)),
    }
}

// Approval prompt — only reached in Normal mode.
// DryRun and Force are fully resolved by the API before returning.
fn handle_pending_plan(pending_plan: Option<api::Plan>) {
    let Some(plan) = pending_plan else { return };

    print!("\nApply this plan? [y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");

    match approve_intent(plan, approved) {
        Ok(output) => {
            print!("✔ ");
            print_lines(output);
        }
        Err(errors) => {
            print!("✗ Error: ");
            print_lines(errors);
        }
    }
}

fn print_lines<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) {
    for item in items {
        println!("{}", item);
    }
}

// Returns true for outputs that are purely informational — no success mark needed.
// A future GUI would use this same classification to decide whether to show a status icon.
fn is_informational(action: &str) -> bool {
    matches!(action, "list" | "help" | "status")
}

// Unified result printer — the only place in the CLI that knows about ✔ and ✗.
// All call sites hand their result here instead of handling marks themselves.
fn print_result(action: &str, result: Result<Vec<String>, Vec<String>>) {
    match result {
        Ok(output) => {
            if !is_informational(action) {
                // FIX: Only print if the first line doesn't already start with a checkmark
                if !output.first().map_or(false, |s| s.starts_with('✔')) {
                    print!("✔ ");
                }
            }
            print_lines(output);
        }
        Err(errors) => {
            print!("✗ Error: ");
            print_lines(errors);
        }
    }
}
