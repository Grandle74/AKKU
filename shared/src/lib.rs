// This Library is used by the "Engine Planner & Executor" and also used by the "Modules State Helpers"
// Since the Modules can't reach and use the Order type directly, this is the shared representation.
//---------------------This Comment isn't verified, yet idk what to say--------------------------------
//----------------------------Engine + Upper layers Types----------------------------------
#[derive(Debug, Clone)]
pub enum Domain {
    Services,
    // Future: Packages, Users, Network...
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Meta(String),   // engine handles — keep as enum variant
    Config,         // engine handles — keep as enum variant
    Custom(String), // module handles — string is fine
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        match s {
            "list" | "help" | "reset" => Action::Meta(s.to_string()),
            "config" | "change" => Action::Config,
            _ => Action::Custom(s.to_string()),
        }
    }
}

//----------------------------Engine + Modules Shared Types--------------------------------
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
