// api/src/service_validator.rs
//
// Temporary stand-in for Services domain conflict validation, to be replaced
// entirely when the module management layer is introduced.
// See ModulesManager.md.
//
// Does not own live system state — only intra-request conflicts are caught
// here; conflicts between requested properties and current machine state will
// surface as runtime errors.

use crate::PropertyValue;
use std::collections::HashMap;

/// Validate all Config properties for the Services domain.
///
/// Only intra-request conflicts are caught (e.g. `running=true` alongside
/// `masked=true`). Conflicts with live system state are out of scope.
pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    for (key, value) in properties {
        validate_property(key, value)?;
    }

    let running = get_bool(properties, "running");
    let enabled = get_bool(properties, "enabled");
    let masked = get_bool(properties, "masked");

    if enabled == Some(true) && masked == Some(true) {
        return Err("Cannot enable and mask a service — masking prevents enabling.".into());
    }
    if running == Some(true) && masked == Some(true) {
        return Err("Cannot start and mask a service — masking prevents starting.".into());
    }

    Ok(())
}

fn validate_property(key: &str, value: &PropertyValue) -> Result<(), String> {
    match key {
        "running" | "enabled" | "masked" => value
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| format!("'{}' expects true/false — see 'service help'", key)),
        _ => Err(format!("Unknown property '{}' — see 'service help'", key)),
    }
}

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    props.get(key)?.as_bool()
}
