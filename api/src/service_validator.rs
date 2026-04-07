// api/src/service_validator.rs
//
// Conflict validation for the Services domain.
//
// This runs BEFORE the engine is called — catching logically impossible
// desired states early, at the API boundary, with a clear user-facing error.
//
// Responsibility: validate property types and cross-property conflicts.
// This file has NO knowledge of current system state — it only reasons
// about what the user asked for.

use crate::PropertyValue;
use std::collections::HashMap;

// TODO: Add action-aware validation (e.g. reject `start` when current state is masked).

/// Validates all Config properties for the Services domain.
///
/// Checks that all keys are recognised, all values have the correct type,
/// and that no logically contradictory combination has been requested.
pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    // Pass 1: per-property type validation.
    for (key, value) in properties {
        validate_property(key, value)?;
    }

    // Pass 2: cross-property conflict detection.
    // These are impossible states regardless of current system state.
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

/// Validates that a single property key is known and its value type is correct.
fn validate_property(key: &str, value: &PropertyValue) -> Result<(), String> {
    match key {
        "running" | "enabled" | "masked" => {
            // We only care that the value IS a bool, not which bool.
            value
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| format!("'{}' expects true/false — see 'service help'", key))
        }
        _ => Err(format!("Unknown property '{}' — see 'service help'", key)),
    }
}

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    props.get(key)?.as_bool()
}
