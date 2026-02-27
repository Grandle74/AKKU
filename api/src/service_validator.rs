use crate::{PropertyValue, get_bool};
use std::collections::HashMap;

pub fn validate(properties: &HashMap<String, PropertyValue>) -> Result<(), String> {
    for (prop, value) in properties {
        if let Err(e) = validate_prop_value(prop, value) {
            return Err(e);
        }
    }
    return Ok(());

    // The following code is commented out - till we implement it
    // It's supposed to validate the changes and order them so that
    // the service is not masked and enabled at the same time
    // or enabled and disabled at the same time

    // let running = get_bool(properties, "running");
    // let enabled = get_bool(properties, "enabled");
    // let masked = get_bool(properties, "masked");

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
}

// This validator checks if the property is valid
// Every property has its own specific valid values
// So... it also validates those values
fn validate_prop_value(prop: &String, value: &PropertyValue) -> Result<(), String> {
    match prop.as_str() {
        "running" | "enabled" | "masked" => {
            if let Some(_) = value.as_bool() {
                return Ok(());
            }
        }
        // "property_name_its_value_is_a_string" => if let Some(_) = value.as_string(){return Ok(());}
        // "property_name_its_value_is_a_number" => if let Some(_) = value.as_number(){return Ok(());}
        _ => return Err(format!("Invalid property: {}, check 'service help'", prop)),
    }
    Err("Invalid property value, check 'service help'".to_string())
}
