// shared_libs/src/lib.rs
//
// Cross-crate type definitions shared by every layer of AKKU.
//
// Does not own execution logic, I/O, or any layer-specific behaviour — if a
// type needs to call into the engine or a module, it belongs elsewhere.
//
// Zero internal dependencies: only std and serde. All other crates depend on
// this one; it must never depend on them.

// ── Domain & Action Types ─────────────────────────────────────────────────────

/// The system domain an Order targets.
///
/// Each variant maps 1-to-1 to a module crate resolved at runtime by
/// `module_resolver`. Adding a domain here requires a matching module.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Domain {
    Services,
    // Future: Packages, Users, Network ...
}

/// The kind of operation an Order requests.
///
/// Three variants cover the full intent space:
/// - `Meta`   — no target, no properties (list, help, reset)
/// - `Config` — declarative desired-state; triggers the Plan/approve flow
/// - `Custom` — imperative single action with a named target (start, stop, …)
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum Action {
    Meta(String),
    Config,
    Custom(String),
}

impl Action {
    /// Return true when the action's output is purely informational.
    ///
    /// Centralised here so the frontend never string-matches action names to
    /// decide whether to prefix output with "✔ ".
    pub fn is_informational(&self) -> bool {
        matches!(self, Action::Meta(_)) || matches!(self, Action::Custom(s) if s == "status")
    }

    /// Return the action name as a plain string.
    ///
    /// Used by `plan_store` to write a flat, human-readable action field
    /// rather than the enum's serialized form.
    pub fn as_str(&self) -> &str {
        match self {
            Action::Custom(s) | Action::Meta(s) => s.as_str(),
            Action::Config => "config",
        }
    }
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        match s {
            "list" | "help" | "reset" => Action::Meta(s.to_string()),
            // Aliases kept narrow deliberately — "config" is the canonical form.
            "config" | "change" | "cfg" => Action::Config,
            _ => Action::Custom(s.to_string()),
        }
    }
}

// ── Property Types ────────────────────────────────────────────────────────────

/// A typed property value used in declarative `Config` orders.
///
/// Keeping this generic means future modules reuse the same property system
/// without CORE changes.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    String(String),
    Number(i64),
}

impl PropertyValue {
    /// Return the inner bool, or `None` if the variant is not `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    /// Return the inner string slice, or `None` if the variant is not `String`.
    pub fn as_str_value(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Return the inner integer, or `None` if the variant is not `Number`.
    pub fn as_number(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

// ── Plan Execution Types ──────────────────────────────────────────────────────

/// An ordered sequence of atomic operations to execute.
pub type Steps = Vec<Step>;

/// A single atomic operation within a Plan.
///
/// Carries enough data to dispatch to the correct module function and to
/// produce a human-readable line in the audit log.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Step {
    pub domain: Domain,
    pub action: Action,
    pub target: String,
    /// Human-readable summary written to plan output and the plan file.
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<String>>,
}

impl Step {
    /// Construct a `Custom` step with a derived description.
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
