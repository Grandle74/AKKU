// shared/src/lib.rs
// Shared types used by both the Engine (planner & executor) and all Modules (state helpers).
// Modules cannot reach Engine types directly, so this crate is the common ground.

// ── Engine + Upper-Layer Types ───────────────────────────────────────────────

#[derive(serde::Deserialize, Debug, Clone)]
pub enum Domain {
    Services,
    // Future: Packages, Users, Network...
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    Meta(String), // No target required — handled entirely by the engine (list, help, reset)
    Config,       // Declarative desired-state — engine builds a Plan, waits for approval
    Custom(String), // Imperative action — dispatched directly to the module (start, stop, ...)
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        match s {
            "list" | "help" | "reset" => Action::Meta(s.to_string()),
            "config" | "change" | "cfg" => Action::Config,
            _ => Action::Custom(s.to_string()),
        }
    }
}

// ── Engine + Module Shared Types ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
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

    pub fn as_string(&self) -> Option<&str> {
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

/// The difference between current and desired state for a service.
/// Produced by `state_helpers::calc()` and consumed by `state_helpers::to_steps()`.
pub struct Delta {
    pub target: Option<String>,
    pub needs_start: bool,
    pub needs_stop: bool,
    pub needs_mask: bool,
    pub needs_unmask: bool,
    pub needs_enable: bool,
    pub needs_disable: bool,
}

pub type Steps = Vec<Step>;

#[derive(serde::Deserialize)]
pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    pub description: String,
}

impl Step {
    pub fn new(domain: Domain, action: &str, target: &str) -> Self {
        Step {
            domain,
            action: Action::Custom(action.to_string()),
            target: target.to_string(),
            description: format!("{} {}", action, target),
        }
    }
}
