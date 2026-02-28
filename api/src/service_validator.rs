// api/src/service_validator.rs
use crate::PropertyValue;
use std::collections::HashMap;

/// Validates all properties: known keys + correct value types.
/// Conflict checks are ready — uncomment to activate.
pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    for (key, value) in properties {
        validate_property(key, value)?;
    }

    // ── Conflict checks ───────────────────────────────────────────────────────
    let running = get_bool(properties, "running");
    let enabled = get_bool(properties, "enabled");
    let masked = get_bool(properties, "masked");

    if enabled == Some(true) && masked == Some(true) {
        return Err("Cannot enable and mask a service — mask prevents enabling.".into());
    }
    if running == Some(true) && masked == Some(true) {
        return Err("Cannot start and mask a service — mask prevents starting.".into());
    }
    // ───────────────────────────────────────────────────────────────────────────

    Ok(())
}

fn validate_property(key: &str, value: &PropertyValue) -> Result<(), String> {
    match key {
        "running" | "enabled" | "masked" => value
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| format!("'{}' expects true/false — check 'service help'", key)),
        // ──── Note for me: ────────────────────────────────────────────────────────────────
        // .map() here is use to discard the boolean.
        // Why? Because validation only cares that it is a boolean, not what it is.
        // ok_or_else() obviously returns Result type
        //───────────────────────────────────────────────────────────────────────────────────
        // Future string property example:
        // "some_key" => value.as_string().map(|_| ()).ok_or_else(|| ...),
        _ => Err(format!("Unknown property '{}' — check 'service help'", key)),
    }
}

//#[allow(dead_code)]
fn get_bool(props: &HashMap<String, PropertyValue>, key: &str) -> Option<bool> {
    props.get(key)?.as_bool()
}
