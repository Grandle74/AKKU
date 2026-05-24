// cli/src/main.rs
//
// commando — the developer/tester reference CLI for AKKU.
//
// This is NOT a consumer-facing tool. It is a reference implementation
// that demonstrates how a frontend should call the API layer. Future
// frontends (GUI, TUI, web) should model their integration on this file.
//
// Dependency rule: this crate imports ONLY `api`. It never imports
// `engine`, `shared_libs`, or any module crate directly. All types
// needed from lower layers are re-exported through `api`.

use api::{
    Action, IntentOutcome, PlanSummary, PropertyValue, RunMode, approve_intent, process_bi_intent,
    process_tri_intent,
};
use rustyline::error::ReadlineError;
use std::collections::HashMap;
use std::io::{self, Write};

mod history;

fn main() {
    println!("Welcome to AKKU (prototype)!");
    println!(
        "This is a prototype of AKKU, a system configuration tool.\n{}",
        "─".repeat(62)
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
                    "history" => history::show_history(),
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

                // One blank line after every command's output, before the next prompt.
                println!();
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

// ── Intent Routing ────────────────────────────────────────────────────────────

fn normalise_domain(s: &str) -> &str {
    match s {
        "service" | "srv" | "services" => "services",
        _ => s,
    }
}

// Routes to the correct API call based on the number of tokens.
//
// Shape → API:
//   1 part  → hint to use domain help
//   2 parts → bi-intent (Meta only)
//   3 parts → tri-intent with no properties (Custom only)
//   4+parts → declarative tri-intent with key=value properties (Config only)
fn handle_intent(parts: &[String], mode: RunMode) {
    let domain = normalise_domain(&parts[0]);

    println!();
    match parts.len() {
        1 => println!("See '{} help' for available commands.", domain),

        2 => {
            let action = &parts[1];
            print_result(action, process_bi_intent(domain, action));
        }

        3 => {
            let action = &parts[1];
            let result = process_tri_intent(
                domain,
                action.clone(),
                parts[2].clone(),
                HashMap::new(),
                &mode,
            );
            handle_outcome(action, result);
        }

        _ => handle_declarative(domain, parts, mode),
    }
}

// Only cfg/config/change are valid declarative action keywords.
// Any other multi-token command is rejected here to give a clear error
// rather than a confusing parse failure deeper in the stack.
fn handle_declarative(domain: &str, parts: &[String], mode: RunMode) {
    let action = parts[1].to_string();
    let target = parts[2].to_string();

    if !matches!(action.as_str(), "cfg" | "config" | "change") {
        println!("✗ Error: Invalid command — see '{} help'", domain);
        return;
    }

    let mut properties: HashMap<String, PropertyValue> = HashMap::new();

    for token in &parts[3..] {
        let Some((key, value)) = token.split_once('=') else {
            println!("✗ Error: Property '{}' must be in key=value format", token);
            return;
        };

        if key.is_empty() {
            println!("✗ Error: Property key cannot be empty");
            return;
        }

        // Reject duplicate keys before sending to the API — HashMap::insert
        // silently drops the first value.
        if properties.contains_key(key) {
            println!("✗ Error: Duplicate property '{}'", key);
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

        properties.insert(key.to_string(), parsed);
    }

    if properties.is_empty() {
        println!("✗ Error: No properties provided — see '{} help'", domain);
        return;
    }

    handle_outcome(
        &action,
        process_tri_intent(domain, action.clone(), target, properties, &mode),
    );
}

// ── Outcome Rendering ─────────────────────────────────────────────────────────

fn handle_outcome(action: &str, result: Result<IntentOutcome, Vec<String>>) {
    match result {
        Ok(outcome) => render_outcome(action, outcome),
        Err(errors) => print_result(action, Err(errors)),
    }
}

// Spacing contract for all match arms: one blank line before each major block
// (plan, approval prompt, status line, rollback block). No trailing blank —
// handle_intent adds one uniformly after every command.
fn render_outcome(action: &str, outcome: IntentOutcome) {
    match outcome {
        IntentOutcome::Immediate(output) => {
            print_result(action, Ok(output));
        }

        IntentOutcome::DryRun { plan } => {
            print_plan(&plan);
        }

        IntentOutcome::RequiresApproval { plan } => {
            print_plan(&plan);
            print!("\nApply this plan? [y/N]: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
            println!();
            render_outcome(action, approve_intent(&plan.id, approved));
        }

        IntentOutcome::Applied { plan, result_text } => {
            println!();
            print_plan(&plan);
            println!("\n⚡ --force: auto-approving plan.");
            print_result(action, Ok(result_text));
        }

        // --force failed — snapshot saved, user can rollback via History.
        IntentOutcome::ApplyFailed {
            plan,
            exec_errors: apply_errors,
        } => {
            println!();
            print_plan(&plan);
            println!("\n⚡ --force: auto-approving plan.");
            println!("✗ Error: Execution failed — snapshot saved for manual rollback.");
            println!();
            print_lines(&apply_errors);
        }

        // No plan here — already printed before the approval prompt.
        IntentOutcome::ApplyFailedRolledBack {
            apply_errors,
            rollback_plan: _,
            result,
        } => {
            println!("\n✗ Error: Execution failed — state restored.");
            println!();
            print_lines(&apply_errors);
            println!();
            print_rollback_block(&result);
        }

        IntentOutcome::ApplyFailedRollbackFailed {
            apply_errors,
            rollback_errors,
            rollback_plan: _,
        } => {
            println!(
                "\n✗ Error: Execution failed — rollback also failed. System state is unknown."
            );
            println!("\nExecution errors:");
            println!();
            print_lines(&apply_errors);
            println!("\nRollback errors:");
            println!();
            print_lines(&rollback_errors);
        }
    }
}

// ── Output Helpers ────────────────────────────────────────────────────────────

// Uses Action::is_informational() to decide whether a success prefix is
// appropriate. This keeps rendering free of hardcoded action name strings.
fn print_result(action_str: &str, result: Result<Vec<String>, Vec<String>>) {
    let action = Action::from(action_str);

    match result {
        Ok(output) => {
            if !action.is_informational() {
                // Only prepend "✔ " if the output doesn't already carry one.
                // Avoids double-prefixing from module-level success messages.
                if !output.first().is_some_and(|s| s.starts_with('✔')) {
                    print!("✔ ");
                }
            }
            print_lines(&output);
        }
        Err(errors) => {
            print!("✗ Error: ");
            print_lines(&errors);
        }
    }
}

// No bullets — these are executor outcome messages, not step descriptions.
// Used for both auto-rollback (ApplyFailedRolledBack) and manual rollback.
fn print_rollback_block(lines: &[String]) {
    let header = "✔ Rollback applied:";
    let divider = "─".repeat(header.len());
    println!("{}", header);
    println!("{}", divider);
    print_lines(lines);
}

fn print_lines(items: &[String]) {
    for item in items {
        println!("{}", item);
    }
}

// The plan is rendered here rather than in the engine — the engine returns
// structured PlanSummary data; this layer owns the display representation.
fn print_plan(plan: &PlanSummary) {
    let header = format!("=== Plan for '{}' ===", plan.target);
    let footer = "=".repeat(header.len());
    println!("{}", header);
    for step in &plan.steps {
        println!("  • {}", step.description);
    }
    println!("{}", footer);
}

// ── Input Parsing ─────────────────────────────────────────────────────────────

// --force takes priority over --dry-run if both are present.
fn parse_flags(input: &str) -> (Vec<String>, RunMode) {
    let mut dry_run = false;
    let mut force = false;

    let parts: Vec<String> = input
        .split_whitespace()
        .filter(|t| match *t {
            "--dry-run" => {
                dry_run = true;
                false
            }
            "--force" => {
                force = true;
                false
            }
            _ => true,
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

// ── Help ──────────────────────────────────────────────────────────────────────

const CLI_HELP: &str = include_str!("../docs/help.txt");

fn show_help() {
    println!("\n{}", CLI_HELP);
}
