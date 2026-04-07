// engine/src/plan_store.rs
//
// All filesystem operations for Plan persistence.
//
// This is the single owner of the ~/.yast3/plans/ directory.
// No other module reads or writes plan files. All plan lifecycle
// transitions (pending → executing → completed/failed/rejected)
// flow through this module.

use std::{fs, path::PathBuf};

fn plans_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".yast3").join("plans")
}

fn plan_path(id: &str) -> PathBuf {
    plans_dir().join(format!("{}.plan.json", id))
}

/// Writes a plan to disk as JSON immediately after creation.
///
/// Called by `approve_plan` in the Normal flow just before the user is
/// prompted — this guarantees an audit record exists even if the process
/// is killed during the approval window.
///
/// The full Step data (domain, action, target) is persisted so the file
/// can later be used for rollback reconstruction.
pub fn save(plan_json: &str, id: &str) -> Result<(), String> {
    fs::create_dir_all(plans_dir()).map_err(|e| e.to_string())?;
    fs::write(plan_path(id), plan_json).map_err(|e| e.to_string())
}

/// Transitions the plan's recorded status field.
///
/// Status lifecycle: `pending` → `executing` → `completed` | `failed` | `rejected`
///
/// Reading and rewriting the whole file is intentional — it keeps the file
/// as a self-contained JSON document rather than a line-appended log,
/// which makes it trivially readable by humans and future tooling.
pub fn update_status(id: &str, status: &str) -> Result<(), String> {
    let content = fs::read_to_string(plan_path(id)).map_err(|e| e.to_string())?;
    let mut data: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    data["status"] = serde_json::json!(status);

    fs::write(
        plan_path(id),
        serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
