pub mod permissions;

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
}

#[async_trait::async_trait(?Send)]
impl acp::Client for BridgedAcpClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        // Extract tool info from the ToolCallUpdate
        let tool = ToolCallInfo {
            name: args.tool_call.fields.title.clone().unwrap_or_default(),
            description: None,
        };

        // Convert ACP PermissionOptions to our bridge PermissionOptions
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

        // Create a oneshot channel for the permission reply
        let (reply_tx, reply_rx) = oneshot::channel();

        // Send the permission request to the main event loop
        let _ = self.evt_tx.send(BridgeEvent::PermissionRequest {
            tool,
            options,
            reply: reply_tx,
        });

        // Wait for the decision from the main thread
        let outcome = reply_rx.await.unwrap_or(PermissionOutcome::Cancelled);

        // Convert our PermissionOutcome back to ACP's RequestPermissionOutcome
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
                let _ = self.evt_tx.send(BridgeEvent::ToolUse {
                    name: tool_call.title.clone(),
                });
            }
            _ => {
                // Ignore other session update variants
            }
        }
        Ok(())
    }
}
