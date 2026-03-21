use crate::agent::registry::AgentOverride;
use crate::client::permissions::PermissionMode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct AcpCliConfig {
    pub default_agent: Option<String>,
    pub default_permissions: Option<PermissionMode>,
    pub timeout: Option<u64>,
    pub format: Option<String>,
    pub agents: Option<HashMap<String, AgentOverride>>,
}

impl AcpCliConfig {
    pub fn load() -> Self {
        dirs::home_dir()
            .map(|h| h.join(".acp-cli").join("config.json"))
            .map(|p| Self::load_from(p))
            .unwrap_or_default()
    }

    pub fn load_from(path: impl AsRef<Path>) -> Self {
        std::fs::read_to_string(path.as_ref())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}
