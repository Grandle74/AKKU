use crate::{Order, PropertyValue, module_resolver::ModuleId};
use serde;
use shared_libs::Steps;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// A generated execution plan.
/// Carries its own module_id so approve_plan() can dispatch without extra parameters.
#[derive(serde::Deserialize)]
pub struct Plan {
    pub id: String,
    pub module_id: ModuleId, // needed by executor to dispatch to the right module
    pub target: String,
    pub output: String,
    pub steps: Steps,
}

impl Plan {
    fn plans_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".yast3").join("plans")
    }

    /// Persists plan to disk immediately after creation.
    /// Steps saved as descriptions only — the live Steps stay in memory for execution.
    pub fn save(&self) -> Result<(), String> {
        fs::create_dir_all(Self::plans_dir()).map_err(|e| e.to_string())?;

        let data = serde_json::json!({
            "id": self.id,
            "target": self.target,
            "status": "pending",
            "steps": self.steps.iter().map(|s| s.description.clone()).collect::<Vec<_>>(),
        });

        let path = Self::plans_dir().join(format!("{}.plan.json", self.id));
        fs::write(
            path,
            serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}

// ── ID Generation ────────────────────────────────────────────────────────────

/// Generates a human-readable, sortable, collision-resistant plan ID.
/// Format: <domain_prefix>_<YYYYMMDD>_<HHMMSS>_<4hex>
/// Example: svc_20260325_143022_a3f2
fn generate_id(prefix: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let ts = chrono::DateTime::from_timestamp(now.as_secs() as i64, 0)
        .unwrap()
        .format("%Y%m%d_%H%M%S")
        .to_string();
    let suffix = format!("{:04x}", (now.as_nanos() % 0x10000) as u16);
    format!("{}_{}_{}", prefix, ts, suffix)
}

fn module_prefix(module: &ModuleId) -> &'static str {
    match module {
        ModuleId::Services => "svc",
        // ModuleId::Network => "net",
    }
}

// ── Public Entry ─────────────────────────────────────────────────────────────

pub fn create_plan(module: &ModuleId, order: &Order) -> Result<Plan, String> {
    let target = order.target.clone().ok_or("No target")?;
    let props = &order.desired_properties;

    let steps: Steps = match module {
        ModuleId::Services => plan_services(target.clone(), props)?,
    };

    let output = format!(
        "=====Plan for '{}':=====\n{}\n==========================",
        target,
        steps
            .iter()
            .map(|s| s.description.clone())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let plan = Plan {
        id: generate_id(module_prefix(module)),
        module_id: module.clone(), // stored so approve_plan() needs no extra params
        target,
        output,
        steps,
    };

    plan.save()?; // persist immediately — audit trail starts at creation
    Ok(plan)
}

// ── Module-Specific Planners ─────────────────────────────────────────────────

fn plan_services(target: String, props: &HashMap<String, PropertyValue>) -> Result<Steps, String> {
    let current_state = services::state_helpers::ServiceCurrentState::new(&target)?;
    let desired_state = services::state_helpers::ServiceDesiredState::from_props(target, props)?;
    let delta = services::state_helpers::calc(&current_state, &desired_state);
    Ok(services::state_helpers::to_steps(&delta))
}
