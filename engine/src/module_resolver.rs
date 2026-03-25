// engine/src/module_resolver.rs
use crate::Domain;

#[derive(serde::Deserialize, Clone)]
pub enum ModuleId {
    Services,
    // Network,
    // Users,
}

pub fn resolve(domain: &Domain) -> Result<ModuleId, String> {
    match domain {
        // ------------------------------------------------------
        // #[cfg(feature = "services")]
        // Without feature flags? `module_resolver` is useless —
        // just match `Domain` directly in the executor.
        // ------------------------------------------------------
        Domain::Services => Ok(ModuleId::Services),
        // Domain::Network => Ok(ModuleId::Network),
        // _ => Err("Failed to resolve module.".to_string()),
    }
}
