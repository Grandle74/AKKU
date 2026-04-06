use api::{
    IntentOutcome, PropertyValue, RunMode, approve_intent, process_bi_intent, process_tri_intent,
};
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

fn handle_intent(parts: &[String], mode: RunMode) {
    let domain = &parts[0];
    match parts.len() {
        1 => println!("See '{} help' for more information.", domain),
        2 => {
            let action = &parts[1];
            print_result(action, process_bi_intent(domain, action));
        }
        3 => {
            let action = &parts[1];
            let res = process_tri_intent(
                domain,
                action.clone(),
                parts[2].clone(),
                HashMap::new(),
                &mode,
            );
            match res {
                Ok(outcome) => render_outcome(action, outcome),
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
        if let Some((key, value)) = token.split_once('=') {
            if key.is_empty() {
                println!("✗ Error: Invalid property");
                return;
            }
            let parsed = match value {
                "true" | "yes" | "1" => PropertyValue::Bool(true),
                "false" | "no" | "0" => PropertyValue::Bool(false),
                _ => value
                    .parse::<i64>()
                    .map(PropertyValue::Number)
                    .unwrap_or_else(|_| PropertyValue::String(value.to_string())),
            };
            if properties.insert(key.to_string(), parsed).is_some() {
                println!("✗ Error: Duplicated property '{}'", key);
                return;
            }
        } else {
            println!("✗ Error: Property must be in key=value format");
            return;
        }
    }

    if properties.is_empty() {
        println!("✗ Error: No properties provided");
        return;
    }

    match process_tri_intent(domain, action.clone(), target, properties, &mode) {
        Ok(outcome) => render_outcome(&action, outcome),
        Err(errors) => print_result(&action, Err(errors)),
    }
}

fn render_outcome(action: &str, outcome: IntentOutcome) {
    match outcome {
        IntentOutcome::Immediate(out) => print_result(action, Ok(out)),
        IntentOutcome::DryRun { plan_text } => print_lines(plan_text),
        IntentOutcome::RequiresApproval { plan, plan_text } => {
            print_lines(plan_text);
            print!("\nApply this plan? [y/N]: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
            print_result(action, approve_intent(plan, approved));
        }
        IntentOutcome::AutoApplied {
            plan_text,
            result_text,
        } => {
            print_lines(plan_text);
            println!("\n⚡ --force: auto-approving plan.");
            print_result(action, Ok(result_text));
        }
        IntentOutcome::ApplyFailed { plan_text, errors } => {
            print_lines(plan_text);
            println!("\n⚡ --force: auto-approving plan.");
            print_result(action, Err(errors));
        }
    }
}

fn print_result(action: &str, result: Result<Vec<String>, Vec<String>>) {
    match result {
        Ok(output) => {
            if !matches!(action, "list" | "help" | "status") {
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

fn print_lines<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) {
    for item in items {
        println!("{}", item);
    }
}

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
