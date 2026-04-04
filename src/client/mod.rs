pub mod permissions;

use std::cell::RefCell;
use std::collections::HashMap;

use agent_client_protocol as acp;
use tokio::sync::{mpsc, oneshot};

use crate::bridge::events::{
    BridgeEvent, PermissionKind, PermissionOption, PermissionOutcome, ToolCallInfo,
};

/// ACP Client implementation that bridges agent protocol events
/// into the internal BridgeEvent channel.
///
/// Handles permission requests by forwarding them through a oneshot channel
/// to the main event loop, and converts session notifications (text chunks,
/// tool calls) into corresponding BridgeEvent variants.
pub struct BridgedAcpClient {
    pub evt_tx: mpsc::UnboundedSender<BridgeEvent>,
    /// Maps tool_call_id → (title, is_read) for in-flight tool calls.
    /// Populated on ToolCall (start), consumed on ToolCallUpdate with Completed/Failed.
    tool_call_state: RefCell<HashMap<String, (String, bool)>>,
}

impl BridgedAcpClient {
    pub fn new(evt_tx: mpsc::UnboundedSender<BridgeEvent>) -> Self {
        Self {
            evt_tx,
            tool_call_state: RefCell::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for BridgedAcpClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        let tool = ToolCallInfo {
            name: args.tool_call.fields.title.clone().unwrap_or_default(),
            description: None,
        };

        let options: Vec<PermissionOption> = args
            .options
            .iter()
            .map(|o| PermissionOption {
                option_id: o.option_id.0.to_string(),
                name: o.name.clone(),
                kind: match o.kind {
                    acp::PermissionOptionKind::AllowOnce
                    | acp::PermissionOptionKind::AllowAlways => PermissionKind::Allow,
                    _ => PermissionKind::Deny,
                },
            })
            .collect();

        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self.evt_tx.send(BridgeEvent::PermissionRequest {
            tool,
            options,
            reply: reply_tx,
        });

        let outcome = reply_rx.await.unwrap_or(PermissionOutcome::Cancelled);

        let acp_outcome = match outcome {
            PermissionOutcome::Selected { option_id } => acp::RequestPermissionOutcome::Selected(
                acp::SelectedPermissionOutcome::new(option_id),
            ),
            PermissionOutcome::Cancelled => acp::RequestPermissionOutcome::Cancelled,
        };

        Ok(acp::RequestPermissionResponse::new(acp_outcome))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        match args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(text_content) = chunk.content {
                    let _ = self.evt_tx.send(BridgeEvent::TextChunk {
                        text: text_content.text,
                    });
                }
            }
            acp::SessionUpdate::ToolCall(tool_call) => {
                // A new tool call has been announced — emit ToolUse for the spinner
                // and record (title, is_read) keyed by tool_call_id for later.
                let is_read = matches!(tool_call.kind, acp::ToolKind::Read);
                let id = tool_call.tool_call_id.0.to_string();
                let title = tool_call.title.clone();
                self.tool_call_state
                    .borrow_mut()
                    .insert(id, (title.clone(), is_read));
                let _ = self.evt_tx.send(BridgeEvent::ToolUse { name: title });
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                // Only act when the tool has finished (Completed or Failed).
                let done = matches!(
                    update.fields.status,
                    Some(acp::ToolCallStatus::Completed) | Some(acp::ToolCallStatus::Failed)
                );
                if done {
                    let id = update.tool_call_id.0.to_string();
                    let (title, is_read) = self
                        .tool_call_state
                        .borrow_mut()
                        .remove(&id)
                        .unwrap_or_default();
                    let output = update
                        .fields
                        .content
                        .as_deref()
                        .map(extract_text_output)
                        .unwrap_or_default();
                    let _ = self.evt_tx.send(BridgeEvent::ToolResult {
                        name: title,
                        output,
                        is_read,
                    });
                }
            }
            _ => {
                // Ignore other session update variants
            }
        }
        Ok(())
    }
}

/// Extract plain-text output from a slice of `ToolCallContent` items.
fn extract_text_output(content: &[acp::ToolCallContent]) -> String {
    content
        .iter()
        .filter_map(|c| {
            if let acp::ToolCallContent::Content(block) = c
                && let acp::ContentBlock::Text(text) = &block.content
            {
                return Some(text.text.clone());
            }
            None
        })
        .collect::<Vec<_>>()
        .join("")
}
