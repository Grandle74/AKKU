// api/src/service_validator.rs
use crate::PropertyValue;
use std::collections::HashMap;

// TODO: Add action-specific validation (e.g. reject "start" on a masked service before unmask).

/// Validates all Config properties for the Services domain:
/// checks that keys are known and values have the correct type,
/// then checks for logically impossible state combinations.
pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    for (key, value) in properties {
        validate_property(key, value)?;
    }

    let running = get_bool(properties, "running");
    let enabled = get_bool(properties, "enabled");
    let masked = get_bool(properties, "masked");

    if enabled == Some(true) && masked == Some(true) {
        return Err("Cannot enable and mask a service — mask prevents enabling.".into());
    }
    if running == Some(true) && masked == Some(true) {
        return Err("Cannot start and mask a service — mask prevents starting.".into());
    }

    Ok(())
}

/// Validates that a single property key is known and its value has the correct type.
fn validate_property(key: &str, value: &PropertyValue) -> Result<(), String> {
    match key {
        "running" | "enabled" | "masked" => {
            // `.map(|_| ())` discards the bool — we only care that it IS a bool, not its value.
            value
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| format!("'{}' expects true/false — check 'service help'", key))
        }
        // Future string property example:
        // "description" => value.as_string().map(|_| ()).ok_or_else(|| ...),
        _ => Err(format!("Unknown property '{}' — check 'service help'", key)),
    }
}

#[allow(dead_code)]
fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    props.get(key)?.as_bool()
}
