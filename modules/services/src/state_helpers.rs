// modules/services/src/state_helpers.rs
//
// Pure state-diffing logic for the Services module.
//
// Responsibility: given a service name and a desired property map,
// produce an ordered list of Steps that will move the service from
// its current state to the desired state — without executing anything.
//
// This file has NO side effects. It only reads from systemctl (via queries)
// and produces data structures. The executor runs the Steps.

use shared_libs::{Delta, Domain, PropertyValue, Step, Steps};
use std::collections::HashMap;
use std::process::Command;

// ── Current State ────────────────────────────────────────────────────────────

/// The live state of a service as reported by systemd.
pub struct ServiceCurrentState {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub masked: bool,
}

impl ServiceCurrentState {
    /// Queries systemd for the current state of `name`.
    ///
    /// Uses `systemctl show` instead of `is-enabled` + `is-active` to get all
    /// properties in a single round-trip. Returns `Err` if the service unit
    /// cannot be found on this system.
    pub fn new(name: &str) -> Result<Self, String> {
        // `systemctl show` is reliable for both file-backed and transient units.
        // Exit code 0 means the unit is known to systemd — even if inactive.
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

        // A unit that systemd has never heard of has LoadState "not-found".
        if load_state == "not-found" {
            return Err(format!("Service '{}' doesn't exist", name));
        }

        Ok(Self {
            name: name.to_string(),
            active: active_state == "active",
            // "static" means the unit has no [Install] section but IS enabled in practice.
            enabled: matches!(unit_file_state.as_str(), "enabled" | "static"),
            masked: unit_file_state == "masked",
        })
    }
}

// ── Desired State ─────────────────────────────────────────────────────────────

/// The state the user declared via Config properties.
///
/// Fields are `Option<bool>` — `None` means "don't care, leave as-is".
/// Only fields set by the user participate in the diff.
pub struct ServiceDesiredState {
    pub name: String,
    pub active: Option<bool>,
    pub enabled: Option<bool>,
    pub masked: Option<bool>,
}

impl ServiceDesiredState {
    pub fn from_props(
        name: String,
        props: &HashMap<String, PropertyValue>,
    ) -> Result<Self, String> {
        Ok(ServiceDesiredState {
            name,
            active: props.get("running").and_then(|v| v.as_bool()),
            enabled: props.get("enabled").and_then(|v| v.as_bool()),
            masked: props.get("masked").and_then(|v| v.as_bool()),
        })
    }
}

// ── Diff & Step Generation ────────────────────────────────────────────────────

/// Compares current vs desired and returns which operations are required.
///
/// A flag is set only when the desired value *differs* from the current value.
/// Fields where desired is `None` are left untouched.
pub fn calc(current: &ServiceCurrentState, desired: &ServiceDesiredState) -> Delta {
    Delta {
        target: Some(current.name.clone()),
        needs_start: desired.active == Some(true) && !current.active,
        needs_stop: desired.active == Some(false) && current.active,
        needs_enable: desired.enabled == Some(true) && !current.enabled,
        needs_disable: desired.enabled == Some(false) && current.enabled,
        needs_unmask: desired.masked == Some(false) && current.masked,
        needs_mask: desired.masked == Some(true) && !current.masked,
    }
}

/// Converts a Delta into an ordered list of Steps.
///
/// Order is critical and intentional:
///   unmask  → must happen before enable/start (a masked unit rejects both)
///   enable  → before start (start without enable works but violates declarative intent)
///   start   → positive state changes before negative
///   stop    → negative changes follow positive
///   disable → after stop
///   mask    → last, so we don't mask something we just started
pub fn to_steps(delta: &Delta) -> Steps {
    let mut steps = vec![];
    let target = delta.target.as_deref().unwrap_or_default();

    if delta.needs_unmask {
        steps.push(Step::new(Domain::Services, "unmask", target));
    }
    if delta.needs_enable {
        steps.push(Step::new(Domain::Services, "enable", target));
    }
    if delta.needs_start {
        steps.push(Step::new(Domain::Services, "start", target));
    }
    if delta.needs_stop {
        steps.push(Step::new(Domain::Services, "stop", target));
    }
    if delta.needs_disable {
        steps.push(Step::new(Domain::Services, "disable", target));
    }
    if delta.needs_mask {
        steps.push(Step::new(Domain::Services, "mask", target));
    }

    steps
}
