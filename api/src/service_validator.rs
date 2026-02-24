use crate::{PropertyValue, get_bool};
use std::collections::HashMap;

pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    let running = get_bool(properties, "running");
    let enabled = get_bool(properties, "enabled");
    let masked = get_bool(properties, "masked");

    // Conflict 1: Can't enable a masked service
    // if enabled == Some(true) && masked == Some(true) {
    //     return Err(
    //         "Conflict: Cannot enable a service while it's masked. Unmask it first.".to_string(),
    //     );
    // }

    // Conflict 2: Can't start a masked service
    // if running == Some(true) && masked == Some(true) {
    //     return Err(
    //         "Conflict: Cannot start a service while it's masked. Unmask it first.".to_string(),
    //     );
    // }

    Ok(())
}
