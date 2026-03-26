// engine/src/plan_store.rs
use std::{fs, path::PathBuf};

fn plans_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".yast3").join("plans")
}

fn plan_path(id: &str) -> PathBuf {
    plans_dir().join(format!("{}.plan.json", id))
}

/// Updates the `status` field of a persisted plan file.
/// Used by `approve_plan()` to track: pending → executing → completed/failed/rejected.
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
