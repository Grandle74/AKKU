// modules/services/src/state_helpers.rs
use shared_libs::{Delta, Domain, PropertyValue, Step, Steps};
use std::collections::HashMap;
use std::process::Command;

/// The current state of a service as reported by systemctl.
pub struct ServiceCurrentState {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub masked: bool,
}

/// The desired state of a service as declared by the user via Config properties.
/// `None` on a field means "don't care — leave it as-is".
pub struct ServiceDesiredState {
    pub name: String,
    pub active: Option<bool>,
    pub enabled: Option<bool>,
    pub masked: Option<bool>,
}

impl ServiceCurrentState {
    pub fn new(name: &str) -> Result<Self, String> {
        if !Self::query(name, &["cat", name]).status.success() {
            return Err(format!("Service '{}' doesn't exist", name));
        }
        let enabled_out = Self::query_stdout(name, &["is-enabled", name]);
        Ok(Self {
            name: name.to_string(),
            active: Self::query_stdout(name, &["is-active", name]) == "active",
            enabled: enabled_out == "enabled" || enabled_out == "static",
            masked: enabled_out == "masked",
        })
    }

    fn query(service: &str, args: &[&str]) -> std::process::Output {
        Command::new("systemctl")
            .args(args)
            .output()
            .unwrap_or_else(|_| panic!("Failed to query service '{}'", service))
    }

    fn query_stdout(service: &str, args: &[&str]) -> String {
        String::from_utf8_lossy(&Self::query(service, args).stdout)
            .trim()
            .to_string()
    }
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

/// Calculates the Delta between current and desired state.
/// Only fields where desired differs from current produce a "needs_*" flag.
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
/// Order matters: unmask before enable, enable before start, etc.
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
