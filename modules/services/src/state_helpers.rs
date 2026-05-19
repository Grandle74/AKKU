// modules/services/src/state_helpers.rs
//
// Pure state-diffing logic for the Services module.
//
// Does not execute anything — produces Steps for the executor to run.
// Does not validate conflicts — that responsibility belongs to the API layer.

use shared_libs::{Domain, PropertyValue, Step, Steps};
use std::collections::HashMap;
use std::process::Command;

// ── Delta ─────────────────────────────────────────────────────────────────────

/// Diff between a service's current state and its desired state.
///
/// Each `needs_*` flag corresponds to one concrete `systemctl` call.
/// Step ordering is fixed in `to_steps` and must not be reordered by callers:
/// unmask → enable → start / stop → disable → mask.
pub struct Delta {
    pub target: Option<String>,
    pub needs_start: bool,
    pub needs_stop: bool,
    pub needs_mask: bool,
    pub needs_unmask: bool,
    pub needs_enable: bool,
    pub needs_disable: bool,
    pub needs_reset_failed: bool,
}

// ── Current State ─────────────────────────────────────────────────────────────

/// The live state of a service as reported by systemd.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ServiceCurrentState {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub masked: bool,
    pub failed: bool,
}

impl ServiceCurrentState {
    /// Queries systemd for the current state of `name`.
    ///
    /// Uses `systemctl show` rather than separate `is-enabled` + `is-active`
    /// calls to get all properties in a single round-trip. Returns `Err` if
    /// the unit is unknown to systemd.
    pub fn new(name: &str) -> Result<Self, String> {
        let output = Command::new("systemctl")
            .args([
                "show",
                name,
                "--property=LoadState,ActiveState,UnitFileState",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(format!("Service '{}' doesn't exist", name));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut load_state = String::new();
        let mut active_state = String::new();
        let mut unit_file_state = String::new();

        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k {
                    "LoadState" => load_state = v.to_string(),
                    "ActiveState" => active_state = v.to_string(),
                    "UnitFileState" => unit_file_state = v.to_string(),
                    _ => {}
                }
            }
        }

        if load_state == "not-found" {
            return Err(format!("Service '{}' doesn't exist", name));
        }

        Ok(Self {
            name: name.to_string(),
            active: active_state == "active",
            // "static" means no [Install] section but the unit is enabled in practice.
            enabled: matches!(unit_file_state.as_str(), "enabled" | "static"),
            masked: unit_file_state == "masked",
            failed: active_state == "failed",
        })
    }
}

// ── Desired State ─────────────────────────────────────────────────────────────

/// The state the user declared via property map.
///
/// Fields are `Option<bool>` — `None` means "don't care, leave as-is".
/// Only fields the user explicitly set participate in the diff.
pub struct ServiceDesiredState {
    pub name: String,
    pub active: Option<bool>,
    pub enabled: Option<bool>,
    pub masked: Option<bool>,
}

impl ServiceDesiredState {
    /// Builds a desired state from the property map supplied by the API layer.
    pub fn from_props(name: &str, props: &HashMap<String, PropertyValue>) -> Result<Self, String> {
        Ok(ServiceDesiredState {
            name: name.to_string(),
            active: props.get("running").and_then(|v| v.as_bool()),
            enabled: props.get("enabled").and_then(|v| v.as_bool()),
            masked: props.get("masked").and_then(|v| v.as_bool()),
        })
    }
}

// ── Diff & Step Generation ────────────────────────────────────────────────────

/// Returns which operations are required to move `current` to `desired`.
///
/// A flag is set only when the desired value differs from the current value.
/// Properties where desired is `None` are left untouched.
pub fn calc(current: &ServiceCurrentState, desired: &ServiceDesiredState) -> Delta {
    Delta {
        target: Some(current.name.clone()),
        needs_start: desired.active == Some(true) && !current.active,
        needs_stop: desired.active == Some(false) && current.active,
        needs_enable: desired.enabled == Some(true) && !current.enabled,
        needs_disable: desired.enabled == Some(false) && current.enabled,
        needs_unmask: desired.masked == Some(false) && current.masked,
        needs_mask: desired.masked == Some(true) && !current.masked,
        // Reset failed state whenever the user is asking for an active or
        // enabled transition — a failed unit silently blocks both.
        needs_reset_failed: current.failed
            && (desired.active.is_some() || desired.enabled.is_some()),
    }
}

/// Converts a Delta into an ordered list of Steps ready for the executor.
///
/// Order is critical:
///   reset   → clean step; clears failed state before any transition attempt
///   unmask  → before enable/start (a masked unit rejects both)
///   enable  → before start
///   start   → positive changes before negative
///   stop    → negative changes follow positive
///   disable → after stop
///   mask    → last, so we don't mask something we just started
///
/// Reset comes first because a failed unit silently blocks enable and start.
pub fn to_steps(delta: &Delta) -> Steps {
    let mut steps = vec![];
    let target = delta.target.as_deref().unwrap_or_default();

    if delta.needs_reset_failed {
        steps.push(Step::new(Domain::Services, "reset", target));
    }
    if delta.needs_unmask {
        steps.push(Step::new(Domain::Services, "unmask", target));
    }
    if delta.needs_enable {
        steps.push(Step::new(Domain::Services, "enable", target));
    }
    if delta.needs_disable {
        steps.push(Step::new(Domain::Services, "disable", target));
    }
    if delta.needs_start {
        steps.push(Step::new(Domain::Services, "start", target));
    }
    if delta.needs_stop {
        steps.push(Step::new(Domain::Services, "stop", target));
    }
    if delta.needs_mask {
        steps.push(Step::new(Domain::Services, "mask", target));
    }

    steps
}
