// engine/src/plan_store.rs
//
// All filesystem operations for Plan persistence.
//
// This is the single owner of the ~/.yast3/plans/ directory.
// No other module reads or writes plan files. All plan lifecycle
// transitions (pending → executing → completed/failed/rejected)
// flow through this module.
//
// File format (example):
// {
//   "id": "svc_20260407_070503_399c",
//   "target": "nginx",
//   "status": "pending",
//   "steps": [
//     { "action": "start", "target": "nginx", "description": "start nginx" }
//   ]
// }
//
// `output` is intentionally absent — it is session display text, not audit data.
// `id` encodes the creation timestamp — no separate `created_at` field is needed.

use crate::planner::Plan;
use std::{fs, path::PathBuf};

fn plans_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".yast3").join("plans")
}

fn plan_path(id: &str) -> PathBuf {
    plans_dir().join(format!("{}.plan.json", id))
}

/// Persists a Plan to disk with status `"pending"`.
///
/// Called by the API layer (via `engine::save_plan`) just before handing
/// the plan back to the frontend — guaranteeing an audit record exists
/// even if the process is killed during the approval window.
///
/// `output` is never written to the file; it is session display text only.
/// Steps are written in a flat format for readability and future tooling.
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

/// Loads a Plan from disk — used internally by approve_plan.
pub(crate) fn load(id: &str) -> Result<Plan, String> {
    let content =
        fs::read_to_string(plan_path(id)).map_err(|_| format!("No plan found for id '{}'", id))?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Builds display lines from a saved plan — used by frontends via api::read_plan.
pub(crate) fn read(id: &str) -> Result<Vec<String>, String> {
    let plan = load(id)?;
    Ok(plan.output)
}

/// Transitions the plan's recorded status field.
///
/// Status lifecycle: `pending` → `executing` → `completed` | `failed` | `rejected`
///
/// Reading and rewriting the whole file is intentional — it keeps the file
/// as a self-contained JSON document rather than a line-appended log,
/// which makes it trivially readable by humans and future tooling.
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

/// Updates the status of a single step by index.
/// Also adds the result output for each step.
/// Called by the executor after each step completes or fails.
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
