use crate::agent::registry::AgentOverride;
use crate::client::permissions::PermissionMode;
use crate::session::scoping::find_git_root;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct AcpCliConfig {
    pub default_agent: Option<String>,
    pub default_permissions: Option<PermissionMode>,
    pub timeout: Option<u64>,
    pub format: Option<String>,
    pub agents: Option<HashMap<String, AgentOverride>>,
}

impl AcpCliConfig {
    /// Load the global config from `~/.acp-cli/config.json`.
    pub fn load() -> Self {
        dirs::home_dir()
            .map(|h| h.join(".acp-cli").join("config.json"))
            .map(Self::load_from)
            .unwrap_or_default()
    }

    /// Load a project-level config by walking from `cwd` up to the git root
    /// and reading `.acp-cli.json` there. Returns `Default` if not found.
    pub fn load_project(cwd: &Path) -> Self {
        find_git_root(cwd)
            .map(|root| root.join(".acp-cli.json"))
            .map(Self::load_from)
            .unwrap_or_default()
    }

    pub fn load_from(path: impl AsRef<Path>) -> Self {
        std::fs::read_to_string(path.as_ref())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Merge project config on top of global. Project wins for non-None fields.
    pub fn merge(self, project: AcpCliConfig) -> AcpCliConfig {
        AcpCliConfig {
            default_agent: project.default_agent.or(self.default_agent),
            default_permissions: project.default_permissions.or(self.default_permissions),
            timeout: project.timeout.or(self.timeout),
            format: project.format.or(self.format),
            agents: match (self.agents, project.agents) {
                (Some(mut base), Some(proj)) => {
                    base.extend(proj);
                    Some(base)
                }
                (base, proj) => proj.or(base),
            },
        }
    }
}
