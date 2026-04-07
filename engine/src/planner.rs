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
/// `output` is skipped during serialization — it is display text assembled
/// for the current session. Each frontend is responsible for rendering its
/// own view of the plan steps. The file format uses `steps` as the source
/// of truth.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Plan {
    pub id: String,
    pub module_id: ModuleId,
    pub target: String,
    #[serde(skip)]
    pub output: Vec<String>,
    pub steps: Steps,
}

// ── ID Generation ─────────────────────────────────────────────────────────────

/// Generates a human-readable, sortable, collision-resistant plan ID.
///
/// Format:  `<domain_prefix>_<YYYYMMDD>_<HHMMSS>_<4hex>`
/// Example: `svc_20260407_143022_a3f2`
///
/// The timestamp embedded in the ID is the canonical creation time —
/// no separate `created_at` field is needed in the plan file.
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

// ── Public Entry ──────────────────────────────────────────────────────────────

/// Builds a Plan for the given Order, or returns `None` if no changes are needed.
///
/// `None` means the service is already at the desired state — the caller
/// should surface this to the user without creating a plan file.
pub fn create_plan(module: &ModuleId, order: &Order) -> Result<Option<Plan>, String> {
    let target = order.target.clone().ok_or("No target provided")?;

    let steps: Steps = match module {
        ModuleId::Services => plan_services(target.clone(), &order.desired_properties)?,
    };

    if steps.is_empty() {
        return Ok(None);
    }

    // Each step description becomes its own line so any frontend can
    // render, join, or format the list however it sees fit.
    let mut output = vec![format!("=== Plan for '{}' ===", target)];
    output.extend(steps.iter().map(|s| format!("  • {}", s.description)));
    output.push("=====================".to_string());

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
