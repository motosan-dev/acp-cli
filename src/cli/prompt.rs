use std::path::PathBuf;
use std::time::Duration;

use crate::bridge::events::{BridgeEvent, PromptResult};
use crate::bridge::{AcpBridge, BridgeCancelHandle};
use crate::client::permissions::{PermissionMode, resolve_permission};
use crate::error::{AcpCliError, Result};
use crate::output::OutputRenderer;
use crate::output::json::JsonRenderer;
use crate::output::quiet::QuietRenderer;
use crate::output::text::TextRenderer;
use crate::session::persistence::SessionRecord;
use crate::session::scoping::{find_git_root, session_dir, session_key};

/// Build a renderer from the format string.
fn make_renderer(output_format: &str) -> Box<dyn OutputRenderer> {
    match output_format {
        "json" => Box::new(JsonRenderer::new()),
        "quiet" => Box::new(QuietRenderer::new()),
        _ => Box::new(TextRenderer::new()),
    }
}

/// Core event loop shared by `run_prompt` and `run_exec`.
///
/// Drives the bridge's event channel concurrently with the prompt reply oneshot.
/// Handles Ctrl-C (graceful cancel on first press, force quit on second) and
/// enforces an optional timeout.
async fn event_loop(
    evt_rx: &mut tokio::sync::mpsc::UnboundedReceiver<BridgeEvent>,
    prompt_reply: tokio::sync::oneshot::Receiver<Result<PromptResult>>,
    cancel: &BridgeCancelHandle,
    renderer: &mut Box<dyn OutputRenderer>,
    permission_mode: &PermissionMode,
    timeout_secs: Option<u64>,
) -> Result<i32> {
    let mut cancel_sent = false;

    // Timeout: either sleep for the given duration, or pend forever.
    let timeout_fut = async {
        match timeout_secs {
            Some(secs) => tokio::time::sleep(Duration::from_secs(secs)).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::pin!(timeout_fut);
    tokio::pin!(prompt_reply);

    loop {
        tokio::select! {
            event = evt_rx.recv() => {
                match event {
                    Some(BridgeEvent::TextChunk { text }) => renderer.text_chunk(&text),
                    Some(BridgeEvent::ToolUse { name }) => renderer.tool_status(&name),
                    Some(BridgeEvent::PermissionRequest { tool, options, reply }) => {
                        let decision = resolve_permission(&tool, &options, permission_mode);
                        if matches!(decision, crate::bridge::PermissionOutcome::Cancelled) {
                            renderer.permission_denied(&tool.name);
                        }
                        let _ = reply.send(decision);
                    }
                    Some(BridgeEvent::SessionCreated { session_id }) => {
                        renderer.session_info(&session_id);
                    }
                    Some(BridgeEvent::PromptDone { .. }) => {
                        // Prompt finished on ACP side; continue draining events.
                    }
                    Some(BridgeEvent::Error { message }) => {
                        renderer.error(&message);
                    }
                    Some(BridgeEvent::AgentExited { code }) => {
                        if let Some(c) = code
                            && c != 0
                        {
                            renderer.error(&format!("agent exited with code {c}"));
                        }
                    }
                    None => break, // channel closed — agent done
                }
            }
            result = &mut prompt_reply => {
                // Prompt RPC completed (oneshot reply from bridge thread).
                renderer.done();
                return match result {
                    Ok(Ok(_)) => Ok(0),
                    Ok(Err(e)) => {
                        renderer.error(&e.to_string());
                        Ok(e.exit_code())
                    }
                    Err(_) => {
                        // Oneshot sender dropped — bridge died unexpectedly
                        renderer.error("bridge connection lost");
                        Ok(1)
                    }
                };
            }
            _ = tokio::signal::ctrl_c() => {
                if cancel_sent {
                    // Second Ctrl+C — force quit
                    return Err(AcpCliError::Interrupted);
                }
                cancel_sent = true;
                eprintln!("\nCancelling... (press Ctrl+C again to force quit)");
                let _ = cancel.cancel().await;
            }
            _ = &mut timeout_fut => {
                eprintln!("\nTimeout after {}s", timeout_secs.unwrap_or(0));
                let _ = cancel.cancel().await;
                tokio::time::sleep(Duration::from_secs(3)).await;
                return Err(AcpCliError::Timeout(timeout_secs.unwrap_or(0)));
            }
        }
    }

    renderer.done();
    Ok(0)
}

/// Run an interactive or piped prompt session.
///
/// Resolves or creates a session, starts the ACP bridge, sends the prompt, and
/// enters the event loop with signal handling and optional timeout.
#[allow(clippy::too_many_arguments)]
pub async fn run_prompt(
    agent_name: &str,
    command: String,
    args: Vec<String>,
    cwd: PathBuf,
    prompt_text: String,
    session_name: Option<String>,
    permission_mode: PermissionMode,
    output_format: &str,
    timeout_secs: Option<u64>,
) -> Result<i32> {
    let mut renderer = make_renderer(output_format);

    // Find or create session record
    let resolved_dir = find_git_root(&cwd).unwrap_or_else(|| cwd.clone());
    let dir_str = resolved_dir.to_string_lossy();
    let sess_name = session_name.as_deref().unwrap_or("");
    let key = session_key(agent_name, &dir_str, sess_name);

    let sess_file = session_dir().join(format!("{key}.json"));
    if SessionRecord::load(&sess_file).ok().flatten().is_none() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let record = SessionRecord {
            id: key.clone(),
            agent: agent_name.to_string(),
            cwd: resolved_dir,
            name: session_name,
            created_at: now,
            closed: false,
        };
        if let Err(e) = record.save(&sess_file) {
            renderer.error(&format!("failed to save session: {e}"));
        }
    }

    // Start bridge
    let mut bridge = AcpBridge::start(command, args, cwd).await?;
    let cancel = bridge.cancel_handle();

    // Send prompt (get oneshot receiver without blocking)
    let prompt_reply = bridge.send_prompt(vec![prompt_text]).await?;

    // Run event loop (borrows evt_rx mutably; cancel handle is separate)
    let result = event_loop(
        &mut bridge.evt_rx,
        prompt_reply,
        &cancel,
        &mut renderer,
        &permission_mode,
        timeout_secs,
    )
    .await;

    let _ = bridge.shutdown().await;
    result
}

/// Run a non-interactive exec command (no session persistence).
pub async fn run_exec(
    command: String,
    args: Vec<String>,
    cwd: PathBuf,
    prompt_text: String,
    permission_mode: PermissionMode,
    output_format: &str,
    timeout_secs: Option<u64>,
) -> Result<i32> {
    let mut renderer = make_renderer(output_format);

    // Start bridge
    let mut bridge = AcpBridge::start(command, args, cwd).await?;
    let cancel = bridge.cancel_handle();

    // Send prompt
    let prompt_reply = bridge.send_prompt(vec![prompt_text]).await?;

    // Run event loop
    let result = event_loop(
        &mut bridge.evt_rx,
        prompt_reply,
        &cancel,
        &mut renderer,
        &permission_mode,
        timeout_secs,
    )
    .await;

    let _ = bridge.shutdown().await;
    result
}
