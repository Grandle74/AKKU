// engine/src/planner.rs
//
// Plan creation: the bridge between an Order and an executable Steps list.
//
// Responsibility: given a module and an Order, query current state,
// diff against desired state, and return an ordered Plan.
// This file does NOT execute steps and does NOT write to disk —
// those concerns belong to the executor and plan_store respectively.

use crate::{Order, PropertyValue, module_resolver::ModuleId};
use shared_libs::Steps;
use std::collections::HashMap;

// ── Plan ─────────────────────────────────────────────────────────────────────

/// A fully resolved execution plan, ready for user approval.
///
/// `Plan` is a pure data carrier — it does not save itself, update statuses,
/// or execute anything. All side effects are handled by `plan_store` and
/// `executor` in the engine layer.
///
/// Both `Serialize` and `Deserialize` are derived so that the full plan
/// (including Step data) can be written to and reconstructed from disk.
/// This is required for the future rollback feature.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Plan {
    pub id: String,
    pub module_id: ModuleId,
    pub target: String,
    /// Human-readable summary shown to the user before approval.
    pub output: String,
    /// The ordered operations to execute. Preserved in the plan file
    /// (not just descriptions) to support future rollback.
    pub steps: Steps,
}

// ── ID Generation ────────────────────────────────────────────────────────────

/// Generates a human-readable, sortable, collision-resistant plan ID.
///
/// Format:  `<domain_prefix>_<YYYYMMDD>_<HHMMSS>_<4hex>`
/// Example: `svc_20260407_143022_a3f2`
///
/// The hex suffix is derived from sub-second nanoseconds, making
/// same-second collisions practically impossible without a UUID library.
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

// ── Public Entry ─────────────────────────────────────────────────────────────

/// Builds a Plan for the given Order, or returns `None` if no changes are needed.
///
/// Returning `None` (instead of a Plan with empty steps) eliminates the need
/// for callers to inspect `plan.steps.is_empty()` and avoids the zombie Plan
/// with `id: String::new()` that previously existed for the empty-steps case.
pub fn create_plan(module: &ModuleId, order: &Order) -> Result<Option<Plan>, String> {
    let target = order.target.clone().ok_or("No target provided")?;

    let steps: Steps = match module {
        ModuleId::Services => plan_services(target.clone(), &order.desired_properties)?,
    };

    // No diff = already at desired state. Signal this cleanly with None.
    if steps.is_empty() {
        return Ok(None);
    }

    let output = format!(
        "=== Plan for '{}' ===\n{}\n=====================",
        target,
        steps
            .iter()
            .map(|s| format!("  • {}", s.description))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(Some(Plan {
        id: generate_id(module_prefix(module)),
        module_id: module.clone(),
        target,
        output,
        steps,
    }))
}

// ── Module-Specific Planners ──────────────────────────────────────────────────

fn plan_services(target: String, props: &HashMap<String, PropertyValue>) -> Result<Steps, String> {
    let current = services::state_helpers::ServiceCurrentState::new(&target)?;
    let desired = services::state_helpers::ServiceDesiredState::from_props(target, props)?;
    let delta = services::state_helpers::calc(&current, &desired);
    Ok(services::state_helpers::to_steps(&delta))
}
