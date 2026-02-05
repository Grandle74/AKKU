use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub timestamp: String,
    pub domain: String,
    pub action: String,
    pub target: String,
    pub success: bool,
    pub messages: Vec<String>,
    pub duration_ms: u64,
}

impl OperationLog {
    pub fn new(domain: &str, action: &str, target: &str) -> Self {
        Self {
            timestamp: Self::current_timestamp(),
            domain: domain.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            success: false,
            messages: Vec::new(),
            duration_ms: 0,
        }
    }

    fn current_timestamp() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();

        let secs = now.as_secs();
        chrono::DateTime::from_timestamp(secs as i64, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    pub fn finish(mut self, result: Result<Vec<String>, Vec<String>>, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        match result {
            Ok(msgs) => {
                self.success = true;
                self.messages = msgs;
            }
            Err(msgs) => {
                self.success = false;
                self.messages = msgs;
            }
        }
        self
    }

    fn log_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".yast3")
    }

    fn log_file() -> PathBuf {
        Self::log_dir().join("operations.log")
    }

    fn latest_file() -> PathBuf {
        Self::log_dir().join("latest_result.json")
    }

    pub fn save(&self) -> std::io::Result<()> {
        fs::create_dir_all(Self::log_dir())?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::log_file())?;

        let json = serde_json::to_string(self)?;
        writeln!(file, "{}", json)?;

        let latest_json = serde_json::to_string_pretty(self)?;
        fs::write(Self::latest_file(), latest_json)?;

        Ok(())
    }

    pub fn read_recent(count: usize) -> Vec<OperationLog> {
        let log_file = Self::log_file();
        if !log_file.exists() {
            return Vec::new();
        }

        let content = fs::read_to_string(log_file).unwrap_or_default();
        let mut logs: Vec<OperationLog> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        logs.reverse();
        logs.truncate(count);
        logs
    }

    pub fn read_latest() -> Option<OperationLog> {
        let latest_file = Self::latest_file();
        if !latest_file.exists() {
            return None;
        }

        let content = fs::read_to_string(latest_file).ok()?;
        serde_json::from_str(&content).ok()
    }
}
