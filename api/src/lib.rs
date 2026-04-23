// api/src/lib.rs
//
// The API layer: the single entry point from the CLI into YaST3.
//
// Responsibilities:
//   1. Parse and validate the request (domain, action, properties) — reject early.
//   2. Call the engine with a well-formed Order.
//   3. Resolve the engine's result into an IntentOutcome based on the run mode.
//   4. Forward approval decisions back to the engine (Trip 2).
//   5. Trigger auto-rollback when execution fails (API thinks, engine acts).
//   6. Expose rollback_intent for future manual rollback from the CLI History flow.
//
// The API does NOT execute systemctl commands and does NOT build Steps.
// It also does NOT render output — that is the CLI's job.
//
// Rollback architecture:
//   Auto-rollback  — triggered HERE by the API when approve_intent detects
//                    execution failure. The engine has no opinion on whether
//                    to roll back — it just executes what it's told.
//   Manual rollback — triggered by rollback_intent(), called by the CLI after
//                     the user picks a plan from History. Not yet wired in the
//                     CLI (History is not implemented), but the full path exists.

use engine::{Domain, Order, approve_plan as engine_approve, execute_order};
pub use engine::{Plan, PropertyValue};
use std::collections::HashMap;

mod service_validator;

pub use engine::Action;
pub use engine::EngineResult as IntentResult;

// ── Public Types ──────────────────────────────────────────────────────────────

/// The resolved outcome of a processed intent, ready for the CLI to render.
pub enum IntentOutcome {
    /// Action completed immediately — display the output lines.
    Immediate(Vec<String>),

    /// Dry-run: plan was generated but nothing was saved or executed.
    DryRun { plan_text: Vec<String> },

    /// Normal flow: plan saved, awaiting explicit user approval.
    RequiresApproval { plan: Plan, plan_text: Vec<String> },

    /// Plan was approved (auto or manual) and executed successfully.
    Applied {
        plan_text: Vec<String>,
        result_text: Vec<String>,
    },

    /// Plan execution failed, and auto-rollback restored the previous state.
    ApplyFailedRolledBack {
        plan_text: Vec<String>,
        exec_errors: Vec<String>,
        rollback_text: Vec<String>,
    },

    /// Plan execution failed, and auto-rollback also failed.
    /// System state is unknown — the user must intervene manually.
    ApplyFailedRollbackFailed {
        plan_text: Vec<String>,
        exec_errors: Vec<String>,
        rollback_errors: Vec<String>,
    },

    /// Manual rollback completed successfully.
    /// plan_text: the rollback plan steps that were executed.
    RolledBack {
        origin_plan_id: String,
        rollback_text: Vec<String>,
    },

    /// Manual rollback itself failed.
    RollbackFailed {
        origin_plan_id: String,
        errors: Vec<String>,
    },
}

/// Controls how a Config intent is handled after planning.
pub enum RunMode {
    Normal, // Prompt user for approval.
    DryRun, // Show the plan, execute nothing.
    Force,  // Auto-approve without prompting.
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Trip 1, bi-intent path: handles Meta actions (list, help, reset) that need
/// no target and no properties. Rejects anything other than a Meta action.
pub fn process_bi_intent(domain_str: &str, action_str: &str) -> Result<Vec<String>, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;
    let action = Action::from(action_str);

    match action {
        Action::Meta(_) => {
            let result = execute_order(Order {
                domain,
                action,
                target: None,
                desired_properties: HashMap::new(),
            })?;
            Ok(result.output)
        }
        _ => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
    }
}

/// Trip 1, tri-intent path: handles all actions that have a target, including
/// imperative Custom actions and declarative Config actions with properties.
pub fn process_tri_intent(
    domain_str: &str,
    action_str: String,
    target: String,
    properties: HashMap<String, PropertyValue>,
    mode: &RunMode,
) -> Result<IntentOutcome, Vec<String>> {
    let action = Action::from(action_str.as_str());
    let domain = validate_request(domain_str, &action, &properties)?;

    let is_config = matches!(action, Action::Config);
    let result = execute_order(Order {
        domain,
        action,
        target: Some(target),
        desired_properties: properties,
    })?;

    // Non-Config actions always execute immediately — the engine returns no plan.
    if !is_config {
        return Ok(IntentOutcome::Immediate(result.output));
    }

    resolve_outcome(result, mode)
}

/// Trip 2: forwards a user's approval decision to the engine.
///
/// On execution failure the API — not the engine — decides to roll back.
/// The engine only executes; the API is the layer that "thinks".
///
/// Returns Ok with success lines on approval + successful execution,
/// or Ok with "Plan rejected." on rejection (not an error — user chose this),
/// or Err with structured rollback outcome embedded in the error lines on failure.
///
/// NOTE: The return type is `Result<Vec<String>, Vec<String>>` — the Err branch
/// carries human-readable lines that the CLI prints as-is. Rollback outcome
/// detail is encoded in those lines. This keeps the CLI call site simple:
///   `print_result(action, approve_intent(plan, approved))`
/// Future frontends that need to distinguish rollback outcomes should instead
/// call `approve_intent_structured()` (not yet added — add when needed).
pub fn approve_intent(plan: Plan, approved: bool) -> Result<Vec<String>, Vec<String>> {
    // Rejection is a clean, non-error outcome — no rollback, no noise.
    if !approved {
        // engine_approve handles the plan_store status update for rejected plans.
        return engine_approve(plan, false).map_err(|e| e); // map_err is identity — Ok("Plan rejected.")
    }

    let plan_id = plan.id.clone();
    let plan_text = plan.output.clone(); // retained for context in rollback error lines

    match engine_approve(plan, true) {
        Ok(output) => Ok(output),

        Err(exec_errors) => {
            // Execution failed — attempt auto-rollback immediately.
            // The engine already marked the plan "failed" in plan_store.
            Err(build_rollback_error_lines(
                &plan_id,
                &plan_text,
                exec_errors,
            ))
        }
    }
}

/// Manual rollback entry point: restores a target to its pre-execution state
/// using the snapshot captured before the original plan ran.
///
/// Called by the CLI History flow once implemented — NOT called during normal
/// approve/execute cycles. The plan_id comes from the user selecting a past plan.
///
/// Returns an IntentOutcome so the CLI can render the rollback result with the
/// same machinery it uses for any other outcome. This also means future GUI/TUI
/// frontends get structured data rather than pre-formatted strings.
pub fn rollback_intent(origin_plan_id: &str) -> IntentOutcome {
    match engine::rollback_plan(origin_plan_id) {
        Ok(rollback_text) => IntentOutcome::RolledBack {
            origin_plan_id: origin_plan_id.to_string(),
            rollback_text,
        },
        Err(errors) => IntentOutcome::RollbackFailed {
            origin_plan_id: origin_plan_id.to_string(),
            errors,
        },
    }
}

// ── Internal Helpers ──────────────────────────────────────────────────────────

/// Validates the request at the API boundary: known domain, legal action for
/// this context, and (for Config) conflict-free properties.
fn validate_request(
    domain_str: &str,
    action: &Action,
    properties: &HashMap<String, PropertyValue>,
) -> Result<Domain, Vec<String>> {
    let domain = parse_domain(domain_str).map_err(|e| vec![e])?;

    match action {
        // Meta actions belong in bi-intent — they should never reach here.
        Action::Meta(_) => Err(vec![format!("Invalid command — see '{} help'", domain_str)]),
        // Config requires at least one property; empty is a user mistake.
        Action::Config if properties.is_empty() => Err(vec![format!(
            "No properties provided — see '{} help'",
            domain_str
        )]),
        // Config with properties: run domain-specific conflict validation.
        Action::Config => {
            validate_conflicts(domain.clone(), properties).map_err(|e| vec![e])?;
            Ok(domain)
        }
        // Custom actions pass through — the module handles unknown action names.
        Action::Custom(_) => Ok(domain),
    }
}

/// Routes a completed engine result to the appropriate IntentOutcome based on run mode.
///
/// Called only for Config actions. By the time we reach here, the engine has
/// already confirmed there is work to do (plan is Some) or not (plan is None).
fn resolve_outcome(result: IntentResult, mode: &RunMode) -> Result<IntentOutcome, Vec<String>> {
    let plan_text = result.output;

    // Engine returned None: the service is already at desired state.
    let Some(plan) = result.pending_plan else {
        return Ok(IntentOutcome::Immediate(plan_text));
    };

    match mode {
        RunMode::DryRun => Ok(IntentOutcome::DryRun { plan_text }),

        RunMode::Force => {
            save_plan(&plan)?;
            // approve_intent handles auto-rollback internally on failure.
            match approve_intent(plan, true) {
                Ok(result_text) => Ok(IntentOutcome::Applied {
                    plan_text,
                    result_text,
                }),
                // The error lines already encode the rollback outcome (see build_rollback_error_lines).
                // We surface the full structured outcome so the CLI can render distinctly.
                Err(error_lines) => resolve_force_failure(plan_text, error_lines),
            }
        }

        RunMode::Normal => {
            // Save before returning to the frontend — guarantees an audit record
            // exists even if the process is killed during the approval window.
            save_plan(&plan)?;
            Ok(IntentOutcome::RequiresApproval { plan, plan_text })
        }
    }
}

/// Classifies a Force-path failure into the correct structured IntentOutcome.
///
/// The error_lines from approve_intent carry a sentinel prefix so we can
/// distinguish "rolled back cleanly" from "rollback also failed" without
/// adding a separate return type to approve_intent.
///
/// This is the only place in the API that reads those sentinels.
fn resolve_force_failure(
    plan_text: Vec<String>,
    error_lines: Vec<String>,
) -> Result<IntentOutcome, Vec<String>> {
    // Sentinels written by build_rollback_error_lines:
    const ROLLED_BACK_SENTINEL: &str = "ROLLBACK:OK";
    const ROLLBACK_FAILED_SENTINEL: &str = "ROLLBACK:FAILED";

    // Split on the sentinel line, which is always first.
    let sentinel = error_lines.first().map(String::as_str).unwrap_or("");

    if sentinel == ROLLED_BACK_SENTINEL {
        let (exec_errors, rollback_text) = split_on_divider(&error_lines[1..]);
        Ok(IntentOutcome::ApplyFailedRolledBack {
            plan_text,
            exec_errors,
            rollback_text,
        })
    } else if sentinel == ROLLBACK_FAILED_SENTINEL {
        let (exec_errors, rollback_errors) = split_on_divider(&error_lines[1..]);
        Ok(IntentOutcome::ApplyFailedRollbackFailed {
            plan_text,
            exec_errors,
            rollback_errors,
        })
    } else {
        // Unexpected — surface raw to avoid swallowing errors silently.
        Err(error_lines)
    }
}

/// Builds the structured error lines returned by `approve_intent` on execution failure.
///
/// Format (lines):
///   ROLLBACK:OK  or  ROLLBACK:FAILED       ← sentinel (first line, parsed by resolve_force_failure)
///   <exec error lines>
///   ---
///   <rollback output or rollback error lines>
///
/// The sentinel + divider approach avoids a separate return type for approve_intent,
/// keeping the CLI call site (`print_result(action, approve_intent(plan, approved))`) simple.
fn build_rollback_error_lines(
    plan_id: &str,
    _plan_text: &[String],
    exec_errors: Vec<String>,
) -> Vec<String> {
    match engine::rollback_plan(plan_id) {
        Ok(rollback_text) => {
            let mut lines = vec!["ROLLBACK:OK".to_string()];
            lines.extend(exec_errors);
            lines.push("---".to_string());
            lines.extend(rollback_text);
            lines
        }
        Err(rollback_errors) => {
            let mut lines = vec!["ROLLBACK:FAILED".to_string()];
            lines.extend(exec_errors);
            lines.push("---".to_string());
            lines.extend(rollback_errors);
            lines
        }
    }
}

/// Splits a slice on the first `"---"` divider line.
/// Returns (before_divider, after_divider). If no divider, everything goes to before.
fn split_on_divider(lines: &[String]) -> (Vec<String>, Vec<String>) {
    if let Some(pos) = lines.iter().position(|l| l == "---") {
        (lines[..pos].to_vec(), lines[pos + 1..].to_vec())
    } else {
        (lines.to_vec(), vec![])
    }
}

/// Persists the plan via the engine's public surface.
///
/// The API never touches plan_store directly — that is a private engine
/// implementation detail. Extracted as a helper to avoid repeating the
/// map_err pattern for both Force and Normal paths.
fn save_plan(plan: &Plan) -> Result<(), Vec<String>> {
    engine::save_plan(plan).map_err(|e| vec![e])
}

fn validate_conflicts(
    domain: Domain,
    properties: &HashMap<String, PropertyValue>,
) -> Result<(), String> {
    match domain {
        Domain::Services => service_validator::validate(properties),
    }
}

fn parse_domain(s: &str) -> Result<Domain, String> {
    match s {
        "service" | "services" | "srv" => Ok(Domain::Services),
        _ => Err(format!("Unknown module '{}' — available: services", s)),
    }
}
