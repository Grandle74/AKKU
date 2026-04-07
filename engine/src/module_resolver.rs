// engine/src/module_resolver.rs
//
// Maps a Domain to the ModuleId used by the executor for dispatch.
//
// This indirection exists so the executor never imports Domain directly —
// it works only with ModuleId. This keeps the engine/module boundary clean
// and makes it easy to add feature-flag-gated modules in the future.

use crate::Domain;

/// Identifies which module implementation handles a given Domain.
/// The executor uses this to call the correct module functions.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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
    }
}
