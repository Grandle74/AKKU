// cli/src/main.rs
//
// commando — the developer/tester reference CLI for YaST3.
//
// This is NOT a consumer-facing tool. It is a reference implementation
// that demonstrates how a frontend should call the API layer. Future
// frontends (GUI, TUI, web) should model their integration on this file.
//
// Responsibilities:
//   - Parse raw input into intent parts and run-mode flags.
//   - Route to the correct API function based on intent shape.
//   - Render IntentOutcome variants to the terminal.
//   - Handle the approval prompt (Trip 2) for the Normal run mode.
//
// Dependency rule: this crate imports ONLY `api`. It never imports
// `engine`, `shared_libs`, or any module crate directly. All types
// needed from lower layers are re-exported through `api`.

use api::{
    Action, IntentOutcome, PropertyValue, RunMode, approve_intent, process_bi_intent,
    process_tri_intent, read_plan,
};
use rustyline::error::ReadlineError;
use std::collections::HashMap;
use std::io::{self, Write};

mod history;

fn main() {
    println!("Welcome to YaST3 (prototype)!");
    println!(
        "This is a prototype of YaST3, a system configuration tool.\n{}",
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

/// Routes a parsed command to the correct API call based on its shape.
///
/// Intent shapes:
///   1 part  → `domain`                        → hint to use help
///   2 parts → `domain action`                 → bi-intent (Meta only)
///   3 parts → `domain action target`          → tri-intent (Custom only)
///   4+ parts→ `domain cfg target key=val ...` → declarative tri-intent (Config only)
fn handle_intent(parts: &[String], mode: RunMode) {
    let domain = &parts[0];

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

/// Handles the declarative path: `domain cfg <target> key=value ...`
///
/// Only `cfg`, `config`, and `change` are valid declarative action keywords.
/// All other multi-token commands are rejected here with a clear error.
fn handle_declarative(domain: &str, parts: &[String], mode: RunMode) {
    let action = parts[1].to_string();
    let target = parts[2].to_string();

    // Guard: only declarative keywords reach this path.
    // Any unknown multi-token command gets a clear rejection instead of
    // a confusing parse error deeper in the stack.
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

        // Reject duplicate keys before sending to the API — HashMap would
        // silently accept the last value, discarding the first.
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

/// Unpacks an API result and routes it to the appropriate render function.
fn handle_outcome(action: &str, result: Result<IntentOutcome, Vec<String>>) {
    match result {
        Ok(outcome) => render_outcome(action, outcome),
        Err(errors) => print_result(action, Err(errors)),
    }
}

/// Renders an IntentOutcome variant to the terminal.
///
/// Spacing rules (all variants):
///   - One blank line before a plan block.
///   - One blank line before the approval prompt.
///   - One blank line before any status line (success, error, banner).
///   - One blank line before a rollback block.
///   - No trailing blank — handle_intent adds it uniformly after every command.
fn render_outcome(action: &str, outcome: IntentOutcome) {
    match outcome {
        IntentOutcome::Immediate(output) => {
            print_result(action, Ok(output));
        }

        IntentOutcome::DryRun { plan_text } => {
            print_lines(&plan_text);
        }

        IntentOutcome::RequiresApproval { plan_id } => {
            if let Ok(plan_text) = read_plan(&plan_id) {
                print_lines(&plan_text);
            }
            print!("\nApply this plan? [y/N]: ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let approved = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");

            println!();
            render_outcome(action, approve_intent(&plan_id, approved));
        }

        // ── Force path ────────────────────────────────────────────────────────
        IntentOutcome::Applied {
            plan_id,
            result_text,
        } => {
            println!();
            if let Ok(plan_text) = read_plan(&plan_id) {
                print_lines(&plan_text);
            }
            println!("\n⚡ --force: auto-approving plan.");
            print_result(action, Ok(result_text));
        }

        // --force failed — snapshot saved, user can rollback via History.
        IntentOutcome::ApplyFailed {
            plan_id,
            exec_errors,
        } => {
            println!();
            if let Ok(plan_text) = read_plan(&plan_id) {
                print_lines(&plan_text);
            }
            println!("\n⚡ --force: auto-approving plan.");
            println!("✗ Error: Execution failed — snapshot saved for manual rollback.");
            println!();
            print_lines(&exec_errors);
        }

        // ── Normal path failures ──────────────────────────────────────────────
        //
        // No plan_text here — already printed before the approval prompt.
        IntentOutcome::ApplyFailedRolledBack {
            exec_errors,
            rollback_text,
        } => {
            println!("\n✗ Error: Execution failed — state restored.");
            println!();
            print_lines(&exec_errors);
            println!();
            print_rollback_block(&rollback_text);
        }

        IntentOutcome::ApplyFailedRollbackFailed {
            exec_errors,
            rollback_errors,
        } => {
            println!(
                "\n✗ Error: Execution failed — rollback also failed. System state is unknown."
            );
            println!("\nExecution errors:");
            println!();
            print_lines(&exec_errors);
            println!("\nRollback errors:");
            println!();
            print_lines(&rollback_errors);
        }

        // ── Manual rollback outcomes (History flow — not yet wired in CLI) ────
        //
        // This is the sole action for this path, so the full block is shown.
        IntentOutcome::RolledBack {
            origin_plan_id: _,
            rollback_text,
        } => {
            println!();
            print_rollback_block(&rollback_text);
        }

        IntentOutcome::RollbackFailed {
            origin_plan_id,
            errors,
        } => {
            println!(
                "✗ Rollback of plan '{}' failed — system state may be inconsistent.",
                origin_plan_id
            );
            println!();
            print_lines(&errors);
        }
    }
}

// ── Output Helpers ────────────────────────────────────────────────────────────

/// Prints a result with a "✔" or "✗" prefix.
///
/// Uses `Action::is_informational()` (defined in shared_libs, re-exported
/// through api) to determine whether a success prefix is appropriate.
/// This keeps the CLI free of hardcoded action name strings.
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

/// Prints a rollback output block with a balanced header/footer.
///
/// `lines` is raw executor output — one line per step result.
/// No bullets — these are outcome messages, not step descriptions.
///
/// Used for both auto-rollback (ApplyFailedRolledBack)
/// and manual rollback (RolledBack).
fn print_rollback_block(lines: &[String]) {
    let header = "↩ Rollback applied:";
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

// ── Input Parsing ─────────────────────────────────────────────────────────────

/// Strips `--dry-run` and `--force` flags from the token stream and
/// returns the cleaned parts alongside the resolved RunMode.
///
/// `--force` takes priority over `--dry-run` if both are present.
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

const CLI_HELP: &str = include_str!("../doc/help.txt");

fn show_help() {
    println!("\n{}", CLI_HELP);
}
