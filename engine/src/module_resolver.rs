// engine/src/module_resolver.rs
//
// Maps a Domain to the ModuleId the executor uses for dispatch.
//
// Does NOT own module installation or the module registry — that belongs
// to a future modules_manager crate, controlled via the API/frontend.
// resolve() is where the engine queries the manager: a Domain only
// dispatches if its module is installed.
//
// ModuleId is intentionally distinct from Domain: Domain is what the
// user requested; ModuleId is what is installed and dispatchable.
// They are 1:1 today but will diverge once optional modules exist.

use crate::Domain;

/// Identifies which module handles a given Domain.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum ModuleId {
    Services,
    // Network,
    // Users,
}

impl ModuleId {
    /// Produces the Domain variant this module handles.
    ///
    /// Used by the snapshot path, which must write domain-typed data
    /// back to the general layer from inside a module-typed context.
    pub fn to_domain(&self) -> Domain {
        match self {
            ModuleId::Services => Domain::Services,
        }
    }

    /// Resolves a Domain to its ModuleId, or errors if no module handles it.
    pub fn resolve(domain: &Domain) -> Result<ModuleId, String> {
        match domain {
            // Without feature flags this match is the only guard —
            // unimplemented domains become compile errors rather than
            // runtime unknowns.
            Domain::Services => Ok(ModuleId::Services),
        }
    }
}
