use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: String,
    pub services: Vec<ServiceState>, // Can add networks, users, etc. later
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub exists: bool,
    pub active: bool,
    pub enabled: bool,
    pub masked: bool,
}

impl SystemSnapshot {
    fn snapshot_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".yast3").join("snapshots")
    }

    fn snapshot_file(name: &str) -> PathBuf {
        Self::snapshot_dir().join(format!("{}.json", name))
    }

    pub fn save(&self, name: &str) -> std::io::Result<()> {
        fs::create_dir_all(Self::snapshot_dir())?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(Self::snapshot_file(name), json)?;
        Ok(())
    }

    pub fn load(name: &str) -> Option<Self> {
        let content = fs::read_to_string(Self::snapshot_file(name)).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn list() -> Vec<String> {
        let dir = Self::snapshot_dir();
        if !dir.exists() {
            return Vec::new();
        }

        fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let name = path.file_stem()?.to_str()?.to_string();
                Some(name)
            })
            .collect()
    }
}

/*
$ yast3 snapshots list
Available snapshots:
  - before_nginx_start (2025-02-07 14:23)
  - before_firewall_config (2025-02-07 14:25)
  - before_user_add (2025-02-07 15:10)

$ yast3 snapshots restore before_nginx_start
=== Rollback Plan ===
1. Stop nginx
2. Disable nginx

Proceed? (y/n)
*/
