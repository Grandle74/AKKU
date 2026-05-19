// engine/src/snapshot.rs
//
// Pre-execution state capture and restoration data for rollback.
//
// save()       — called in approve_plan before execution begins.
// load()       — called by the rollback path, given a plan ID.
// into_order() — translates a loaded snapshot back into a Config Order
//                so the planner can produce a restoration plan as usual.

use crate::Order;
use shared_libs::{Action, Domain, PropertyValue};
use std::{collections::HashMap, fs, path::PathBuf};

fn snapshot_path(plan_id: &str) -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".akku/snapshots")
        .join(format!("{}.snapshot.json", plan_id))
}

/// Captured pre-execution state for a single plan target.
///
/// `state` is stored as an untyped JSON value rather than a domain-specific
/// struct so that one file format works across all modules without a shared
/// trait or enum wrapper. `to_properties` handles the per-domain translation
/// back to a typed property map.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub plan_id: String,
    pub domain: Domain,
    pub target: String,
    pub state: serde_json::Value,
}

impl Snapshot {
    /// Translates the captured state back into a Config Order.
    ///
    /// Expressing the restoration as a declarative property map means the
    /// planner generates the correct steps without any rollback-specific logic —
    /// rollback reuses the exact same planning path as a forward change.
    pub fn into_order(self) -> Result<Order, String> {
        let properties = to_properties(&self.domain, &self.state)?;
        Ok(Order {
            domain: self.domain,
            action: Action::Config,
            target: Some(self.target),
            desired_properties: properties,
            mode: None, // rollback path stamps mode before calling plan_store::save
        })
    }
}

/// Captures the target's live state and writes it to disk.
///
/// Must succeed before execution begins — `approve_plan` marks the plan
/// "aborted" and surfaces the error if this returns Err.
pub fn save(plan_id: &str, domain: &Domain, target: &str) -> Result<(), String> {
    let state = capture_state(domain, target)?;
    let snapshot = Snapshot {
        plan_id: plan_id.to_string(),
        domain: domain.clone(),
        target: target.to_string(),
        state,
    };
    let path = snapshot_path(plan_id);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(
        &path,
        serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// Reads a snapshot from disk by plan ID.
pub fn load(plan_id: &str) -> Result<Snapshot, String> {
    let content = fs::read_to_string(snapshot_path(plan_id))
        .map_err(|_| format!("No snapshot found for plan '{}'", plan_id))?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn capture_state(domain: &Domain, target: &str) -> Result<serde_json::Value, String> {
    match domain {
        Domain::Services => {
            let current = services::state_helpers::ServiceCurrentState::new(target)?;
            serde_json::to_value(&current).map_err(|e| e.to_string())
        }
    }
}

// Translates an untyped JSON state blob back to a typed property map.
// The roundtrip through the domain's typed struct validates that the
// stored state is still structurally sound before feeding it to the planner.
fn to_properties(
    domain: &Domain,
    state: &serde_json::Value,
) -> Result<HashMap<String, PropertyValue>, String> {
    match domain {
        Domain::Services => {
            let current: services::state_helpers::ServiceCurrentState =
                serde_json::from_value(state.clone()).map_err(|e| e.to_string())?;
            let mut props = HashMap::new();
            props.insert("running".to_string(), PropertyValue::Bool(current.active));
            props.insert("enabled".to_string(), PropertyValue::Bool(current.enabled));
            props.insert("masked".to_string(), PropertyValue::Bool(current.masked));
            Ok(props)
        } // Future: add cases for other domains as needed.
    }
}
