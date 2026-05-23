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

use shared_libs::{PlanSummary, StepSummary};

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
    let mut plan = plan.clone();
    plan.status = Some("pending".to_string());
    fs::write(
        plan_path(&plan.id),
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
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

/// Transitions the plan's recorded status field.
///
/// Status lifecycle: `pending` → `executing` → `completed` | `failed` | `rejected`
///
// Read-and-rewrite rather than append keeps the file a self-contained JSON document.
pub(crate) fn update_status(id: &str, status: &str) -> Result<(), String> {
    let mut plan = load(id)?;
    plan.status = Some(status.to_string());
    fs::write(
        plan_path(id),
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
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
    let mut plan = load(id)?;
    let step = plan
        .steps
        .get_mut(step_index)
        .ok_or_else(|| format!("No step at index {}", step_index))?;
    step.status = Some(status.to_string());
    step.output = Some(output.to_vec());
    fs::write(
        plan_path(id),
        serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

// ── Plan History ──────────────────────────────────────────────────────────────

/// Returns all persisted plans as ready-to-consume summaries, sorted oldest-first.
///
/// Malformed or unreadable files are silently skipped — a corrupted plan
/// does not block the rest of the history from loading.
pub(crate) fn list_plans() -> Result<Vec<PlanSummary>, String> {
    let dir = plans_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut summaries: Vec<PlanSummary> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .filter_map(|e| {
            let content = fs::read_to_string(e.path()).ok()?;
            let plan: Plan = serde_json::from_str(&content).ok()?;
            Some(to_summary(plan))
        })
        .collect();

    // IDs are lexicographically sortable by timestamp — no date parsing needed here.
    summaries.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(summaries)
}

/// Converts a persisted Plan into a frontend-consumable PlanSummary.
pub(crate) fn to_summary(plan: Plan) -> PlanSummary {
    PlanSummary {
        date: date_from_id(&plan.id),
        summary: build_summary(&plan),
        steps: plan
            .steps
            .into_iter()
            .map(|s| StepSummary {
                description: s.description,
                status: s.status,
            })
            .collect(),
        id: plan.id,
        target: plan.target,
        status: plan.status.unwrap_or_else(|| "pending".to_string()),
        rollback_of: plan.rollback_of,
        mode: plan.mode,
    }
}

/// Extracts a human-readable timestamp from a plan ID.
///
/// ID format: `<prefix>_<YYYYMMDD>_<HHMMSS>_<hex>`
/// Example:   `svc_20260407_143022_a3f2` → `2026-04-07 14:30`
///
/// Returns the raw ID on malformed input rather than failing — history
/// display degrades gracefully.
fn date_from_id(id: &str) -> String {
    let parts: Vec<&str> = id.split('_').collect();

    if parts.len() < 4 {
        return id.to_string();
    }

    let date = parts[1]; // YYYYMMDD
    let time = parts[2]; // HHMMSS

    if date.len() == 8 && time.len() == 6 {
        format!(
            "{}-{}-{} {}:{}",
            &date[0..4],
            &date[4..6],
            &date[6..8],
            &time[0..2],
            &time[2..4],
        )
    } else {
        id.to_string()
    }
}

/// Builds a one-line action summary from a plan's steps.
///
/// Examples:
///   "start nginx"                 (1 step)
///   "enable, start nginx"         (2 steps)
///   "rolled back svc_20260407_…"  (rollback plan)
fn build_summary(plan: &Plan) -> String {
    if let Some(origin) = &plan.rollback_of {
        return format!("rolled back {}", origin);
    }

    if plan.steps.is_empty() {
        return "—".into();
    }

    let mut seen = std::collections::HashSet::new();
    let actions: Vec<&str> = plan
        .steps
        .iter()
        .map(|s| s.action.as_str())
        .filter(|a| seen.insert(*a))
        .collect();

    format!("{} {}", actions.join(", "), plan.target)
}
