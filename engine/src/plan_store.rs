// engine/src/plan_store.rs
//
// All filesystem operations for Plan persistence.
//
// This is the sole owner of ~/.akku/plans/. No other module reads or writes
// plan files. All plan status transitions flow through this module.
//
// File format: JSON objects keyed by the plan's own fields.
// `output` is absent — it is session display text, not audit data.
// `id` encodes the creation timestamp, so no separate `created_at` is needed.

use crate::planner::Plan;
use std::{fs, path::PathBuf};

fn plans_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".akku").join("plans")
}

fn plan_path(id: &str) -> PathBuf {
    plans_dir().join(format!("{}.plan.json", id))
}

/// Persists a Plan to disk with status `"pending"`.
pub(crate) fn save(plan: &Plan) -> Result<(), String> {
    fs::create_dir_all(plans_dir()).map_err(|e| e.to_string())?;
    let mut data = serde_json::to_value(plan).map_err(|e| e.to_string())?;
    data["status"] = serde_json::json!("pending");
    fs::write(
        plan_path(&plan.id),
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// Deserializes a Plan from disk by ID.
///
/// This is the handoff between Trip 1 and Trip 2 of the plan/approve flow —
/// the in-memory plan from `execute_order` is gone by the time `approve_plan` runs.
pub(crate) fn load(id: &str) -> Result<Plan, String> {
    let content =
        fs::read_to_string(plan_path(id)).map_err(|_| format!("No plan found for id '{}'", id))?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Produces display lines from a saved plan — the engine's display gate for frontends.
pub(crate) fn read(id: &str) -> Result<Vec<String>, String> {
    let plan = load(id)?;
    Ok(plan.output)
}

/// Transitions the plan's recorded status field.
///
/// Status lifecycle: `pending` → `executing` → `completed` | `failed` | `rejected`
///
// Read-and-rewrite rather than append keeps the file a self-contained JSON document.
pub(crate) fn update_status(id: &str, status: &str) -> Result<(), String> {
    let content = fs::read_to_string(plan_path(id)).map_err(|e| e.to_string())?;
    let mut data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    data["status"] = serde_json::json!(status);

    fs::write(
        plan_path(id),
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// Records the result of a single step in the on-disk plan.
///
/// Called by the executor after each step.
pub(crate) fn update_step_status(
    id: &str,
    step_index: usize,
    status: &str,
    output: &[String],
) -> Result<(), String> {
    let content = fs::read_to_string(plan_path(id)).map_err(|e| e.to_string())?;
    let mut data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    data["steps"][step_index]["status"] = serde_json::json!(status);
    data["steps"][step_index]["output"] = serde_json::json!(output);

    fs::write(
        plan_path(id),
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
