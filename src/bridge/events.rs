use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct PromptResult {
    pub content: String,
    pub stop_reason: String,
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: PermissionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionKind {
    Allow,
    Deny,
}

#[derive(Debug)]
pub enum PermissionOutcome {
    Selected { option_id: String },
    Cancelled,
}

pub enum BridgeEvent {
    TextChunk {
        text: String,
    },
    ToolUse {
        name: String,
    },
    /// Emitted when a tool call completes and returns output.
    /// `is_read` is true when the tool's kind is `ToolKind::Read` (file read).
    ToolResult {
        name: String,
        output: String,
        is_read: bool,
    },
    PermissionRequest {
        tool: ToolCallInfo,
        options: Vec<PermissionOption>,
        reply: oneshot::Sender<PermissionOutcome>,
    },
    SessionCreated {
        session_id: String,
    },
    PromptDone {
        stop_reason: String,
    },
    Error {
        message: String,
    },
    AgentExited {
        code: Option<i32>,
    },
}
