use serde::Deserialize;

use crate::bridge::{PermissionKind, PermissionOption, PermissionOutcome, ToolCallInfo};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    ApproveAll,
    #[default]
    ApproveReads,
    DenyAll,
}

/// Resolve a permission request based on the current permission mode.
///
/// For `ApproveAll`, automatically selects the first allow option.
/// For `ApproveReads`, only auto-approves read-only tools; write tools are cancelled.
/// For `DenyAll`, always cancels.
pub fn resolve_permission(
    tool: &ToolCallInfo,
    options: &[PermissionOption],
    mode: &PermissionMode,
) -> PermissionOutcome {
    match mode {
        PermissionMode::ApproveAll => select_first_allow(options),
        PermissionMode::ApproveReads => {
            if is_read_only_tool(&tool.name) {
                select_first_allow(options)
            } else {
                PermissionOutcome::Cancelled
            }
        }
        PermissionMode::DenyAll => PermissionOutcome::Cancelled,
    }
}

/// Returns true if the tool name corresponds to a read-only operation.
pub fn is_read_only_tool(name: &str) -> bool {
    matches!(
        name,
        "Read" | "Glob" | "Grep" | "WebSearch" | "WebFetch" | "LSP"
    )
}

fn select_first_allow(options: &[PermissionOption]) -> PermissionOutcome {
    options
        .iter()
        .find(|o| o.kind == PermissionKind::Allow)
        .map(|o| PermissionOutcome::Selected {
            option_id: o.option_id.clone(),
        })
        .unwrap_or(PermissionOutcome::Cancelled)
}
