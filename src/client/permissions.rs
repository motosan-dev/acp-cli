use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    ApproveAll,
    ApproveReads,
    DenyAll,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::ApproveReads
    }
}
