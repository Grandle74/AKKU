// shared_libs/src/lib.rs
//
// Cross-crate type definitions shared by every layer of YaST3.
//
// Dependency rule: this crate has ZERO internal dependencies — it only uses
// the standard library and serde. All other crates depend on this one.
// Never pull engine or module types into here.

// ── Core Intent Types ────────────────────────────────────────────────────────

/// The module (system domain) an Order targets.
/// Each Domain maps 1-to-1 to a Module crate via `module_resolver`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Domain {
    Services,
    // Future: Packages, Users, Network ...
}

/// What kind of operation an Order requests.
///
/// Three variants cover the entire intent space:
/// - `Meta`   — no target, no properties (list, help, reset)
/// - `Config` — declarative desired-state with properties; triggers Plan/approve flow
/// - `Custom` — imperative single action with a target (start, stop, status …)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    Meta(String),
    Config,
    Custom(String),
}

impl Action {
    /// Returns true for actions whose output is purely informational and should
    /// NOT receive the leading "✔ " success prefix in the Frontend.
    ///
    /// Centralised here so the Frontend never needs to string-match action names.
    pub fn is_informational(&self) -> bool {
        matches!(self, Action::Meta(_)) || matches!(self, Action::Custom(s) if s == "status")
    }

    /// Returns the action name as a plain string.
    /// Used by plan_store to write a flat, human-readable action field
    /// instead of the enum's serialized form.
    pub fn as_str(&self) -> &str {
        match self {
            Action::Custom(s) => s.as_str(),
            Action::Meta(s) => s.as_str(),
            Action::Config => "config",
        }
    }
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        match s {
            // Meta: no target required, handled entirely inside the engine dispatcher.
            "list" | "help" | "reset" => Action::Meta(s.to_string()),
            // Config: declarative desired-state — engine builds a Plan, awaits approval.
            "config" | "change" | "cfg" => Action::Config,
            // Everything else is a Custom imperative action routed to the module.
            _ => Action::Custom(s.to_string()),
        }
    }
}

// ── Shared Engine / Module Types ─────────────────────────────────────────────

/// A typed property value used in declarative Config orders.
///
/// Keeps the property system generic so any future module can reuse it
/// without requiring engine changes.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    String(String),
    Number(i64),
}

impl PropertyValue {
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn as_str_value(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

// ── Plan Execution Types ─────────────────────────────────────────────────────

/// The diff between a service's current state and its desired state.
///
/// Produced by `state_helpers::calc()`. Each `needs_*` flag represents
/// one concrete systemctl operation. The ordering in `to_steps()` is
/// deliberately fixed: unmask → enable → start / stop → disable → mask.
pub struct Delta {
    pub target: Option<String>,
    pub needs_start: bool,
    pub needs_stop: bool,
    pub needs_mask: bool,
    pub needs_unmask: bool,
    pub needs_enable: bool,
    pub needs_disable: bool,
    pub needs_reset_failed: bool,
}

/// An ordered list of atomic operations to execute in sequence.
pub type Steps = Vec<Step>;

/// A single atomic operation within a Plan.
///
/// Carries enough data to be dispatched to the correct module function
/// and to reconstruct a human-readable description for the audit log.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    /// Human-readable summary used in plan output and saved to the plan file.
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<String>>,
}

impl Step {
    pub fn new(domain: Domain, action: &str, target: &str) -> Self {
        Step {
            description: format!("{} {}", action, target),
            domain,
            action: Action::Custom(action.to_string()),
            target: target.to_string(),
            status: None,
            output: None,
        }
    }
}
