// engine/src/planner.rs
//
// Produces an ordered Plan from an Order by diffing current against desired state.
//
// Does NOT execute steps and does NOT write to disk — those belong to
// the executor and plan_store respectively.

use crate::{Order, PropertyValue, module_resolver::ModuleId};
use shared_libs::Steps;
use std::collections::HashMap;

// ── Plan ─────────────────────────────────────────────────────────────────────

/// A fully resolved execution plan, ready for user approval.
///
/// `Plan` is a pure data carrier — it does not save itself, update statuses,
/// or execute anything. All side effects are handled by `plan_store` and
/// `executor`.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Plan {
    pub id: String,
    pub(crate) module_id: ModuleId,
    pub target: String,
    // `serde(default)` tolerates plan files written before this field existed.
    #[serde(default)]
    pub(crate) status: Option<String>,
    pub steps: Steps,
    /// "normal", "force", or "rollback". None on freshly planned in-memory plans —
    /// the API layer stamps this before saving.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mode: Option<String>,
    /// Set when this plan is itself a rollback. Signals `approve_plan` to skip
    /// snapshot capture — capturing here would overwrite the pre-change snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_of: Option<String>,
}

// ── ID Generation ─────────────────────────────────────────────────────────────

/// Generates a human-readable, sortable, collision-resistant plan ID.
///
/// Format:  `<domain_prefix>_<YYYYMMDD>_<HHMMSS>_<4hex>`
/// Example: `svc_20260407_143022_a3f2`
///
/// The timestamp is the canonical creation time — no separate `created_at`
/// field is needed. The hex suffix is derived from sub-second nanoseconds,
/// making same-second collisions practically impossible without a UUID library.
fn generate_id(prefix: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();

    let timestamp = chrono::DateTime::from_timestamp(now.as_secs() as i64, 0)
        .unwrap()
        .format("%Y%m%d_%H%M%S")
        .to_string();

    let suffix = format!("{:04x}", (now.as_nanos() % 0x10000) as u16);
    format!("{}_{}_{}", prefix, timestamp, suffix)
}

fn module_prefix(module: &ModuleId) -> &'static str {
    match module {
        ModuleId::Services => "svc",
        // ModuleId::Network => "net",
    }
}

// ── Public Entry ──────────────────────────────────────────────────────────────

/// Builds a Plan for the given Order, or returns `None` if no changes are needed.
///
/// `None` means the target is already at the desired state — the caller
/// should surface this to the user without creating a plan file.
pub fn create_plan(module: &ModuleId, order: &Order) -> Result<Option<Plan>, String> {
    let target = order.target.as_deref().ok_or("No target provided")?;
    let steps: Steps = match module {
        ModuleId::Services => plan_services(target, &order.desired_properties)?,
    };

    if steps.is_empty() {
        return Ok(None);
    }

    Ok(Some(Plan {
        id: generate_id(module_prefix(module)),
        module_id: module.clone(),
        target: target.to_string(),
        status: None,
        steps,
        status: "pending".to_string(),
        rollback_of: None,
        mode: None,
    }))
}

// ── Module-Specific Planners ──────────────────────────────────────────────────

fn plan_services(target: &str, props: &HashMap<String, PropertyValue>) -> Result<Steps, String> {
    let current = services::state_helpers::ServiceCurrentState::new(target)?;
    let desired = services::state_helpers::ServiceDesiredState::from_props(target, props)?;
    let delta = services::state_helpers::calc(&current, &desired);
    Ok(services::state_helpers::to_steps(&delta))
}
