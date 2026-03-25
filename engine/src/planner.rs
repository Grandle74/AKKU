use crate::{Order, PropertyValue, module_resolver::ModuleId};
use shared_libs::Steps;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Plan {
    pub id: String, // added
    pub target: String,
    pub output: String,
    pub steps: Steps,
}

impl Plan {
    fn plans_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".yast3").join("plans")
    }

    pub fn save(&self) -> Result<(), String> {
        fs::create_dir_all(Self::plans_dir()).map_err(|e| e.to_string())?;

        let data = serde_json::json!({
            "id": self.id,
            "target": self.target,
            "status": "pending",
            "steps": self.steps
                .iter()
                .map(|s| s.description.clone())
                .collect::<Vec<_>>(),
        });

        let path = Self::plans_dir().join(format!("{}.plan.json", self.id));
        fs::write(
            path,
            serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}

// ── ID generation ────────────────────────────────────────────────────────────

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

// ── Public entry point ───────────────────────────────────────────────────────

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
        id: generate_id(module_prefix(module)), // added
        target,
        output,
        steps,
    };

    plan.save()?; // added - saves right after creation
    Ok(plan)
}

// ── Module-specific planners ─────────────────────────────────────────────────

fn plan_services(target: String, props: &HashMap<String, PropertyValue>) -> Result<Steps, String> {
    let current_state = services::state_helpers::ServiceCurrentState::new(&target)?;
    let desired_state = services::state_helpers::ServiceDesiredState::from_props(target, props)?;
    let delta = services::state_helpers::calc(&current_state, &desired_state);
    Ok(services::state_helpers::to_steps(&delta))
}
