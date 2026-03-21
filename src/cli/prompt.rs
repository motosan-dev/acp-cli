use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::bridge::events::{BridgeEvent, PromptResult};
use crate::bridge::{AcpBridge, BridgeCancelHandle};
use crate::client::permissions::{PermissionMode, resolve_permission};
use crate::error::{AcpCliError, Result};
use crate::output::OutputRenderer;
use crate::output::json::JsonRenderer;
use crate::output::quiet::QuietRenderer;
use crate::output::text::TextRenderer;
use crate::queue::client::QueueClient;
use crate::queue::ipc::start_ipc_server;
use crate::queue::lease::LeaseFile;
use crate::queue::owner::QueueOwner;
use crate::session::history::{ConversationEntry, append_entry};
use crate::session::persistence::SessionRecord;
use crate::session::pid;
use crate::session::scoping::{find_git_root, session_dir, session_key};

/// RAII guard that removes the PID file when dropped, ensuring cleanup even on
/// early returns or panics.
struct PidGuard {
    session_key: String,
}

impl PidGuard {
    fn new(session_key: &str) -> std::io::Result<Self> {
        pid::write_pid(session_key)?;
        Ok(Self {
            session_key: session_key.to_string(),
        })
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = pid::remove_pid(&self.session_key);
    }
}

/// Build a renderer from the format string.
fn make_renderer(output_format: &str) -> Box<dyn OutputRenderer> {
    match output_format {
        "json" => Box::new(JsonRenderer::new()),
        "quiet" => Box::new(QuietRenderer::new()),
        _ => Box::new(TextRenderer::new()),
    }
}

/// Result from the event loop, including the exit code and collected assistant text.
struct EventLoopResult {
    exit_code: i32,
    assistant_text: String,
    /// The ACP session ID emitted by the bridge (if any).
    acp_session_id: Option<String>,
}

/// Core event loop shared by `run_prompt` and `run_exec`.
///
/// Drives the bridge's event channel concurrently with the prompt reply oneshot.
/// Handles Ctrl-C (graceful cancel on first press, force quit on second) and
/// enforces an optional timeout. Collects all TextChunk content for conversation logging.
async fn event_loop(
    evt_rx: &mut tokio::sync::mpsc::UnboundedReceiver<BridgeEvent>,
    prompt_reply: tokio::sync::oneshot::Receiver<Result<PromptResult>>,
    cancel: &BridgeCancelHandle,
    renderer: &mut Box<dyn OutputRenderer>,
    permission_mode: &PermissionMode,
    timeout_secs: Option<u64>,
) -> Result<EventLoopResult> {
    let mut cancel_sent = false;
    let mut collected_text = String::new();
    let mut acp_session_id: Option<String> = None;

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
                    Some(BridgeEvent::TextChunk { text }) => {
                        collected_text.push_str(&text);
                        renderer.text_chunk(&text);
                    }
                    Some(BridgeEvent::ToolUse { name }) => renderer.tool_status(&name),
                    Some(BridgeEvent::PermissionRequest { tool, options, reply }) => {
                        let decision = resolve_permission(&tool, &options, permission_mode);
                        if matches!(decision, crate::bridge::PermissionOutcome::Cancelled) {
                            renderer.permission_denied(&tool.name);
                        }
                        let _ = reply.send(decision);
                    }
                    Some(BridgeEvent::SessionCreated { session_id }) => {
                        acp_session_id = Some(session_id.clone());
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
                    Ok(Ok(_)) => Ok(EventLoopResult { exit_code: 0, assistant_text: collected_text, acp_session_id: acp_session_id.clone() }),
                    Ok(Err(e)) => {
                        renderer.error(&e.to_string());
                        Ok(EventLoopResult { exit_code: e.exit_code(), assistant_text: collected_text, acp_session_id: acp_session_id.clone() })
                    }
                    Err(_) => {
                        // Oneshot sender dropped — bridge died unexpectedly
                        renderer.error("bridge connection lost");
                        Ok(EventLoopResult { exit_code: 1, assistant_text: collected_text, acp_session_id: acp_session_id.clone() })
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
    Ok(EventLoopResult {
        exit_code: 0,
        assistant_text: collected_text,
        acp_session_id,
    })
}

/// Run an interactive or piped prompt session.
///
/// Resolves or creates a session, starts the ACP bridge, sends the prompt, and
/// enters the event loop with signal handling and optional timeout. After the
/// first prompt completes, becomes the queue owner — listening on a Unix socket
/// for subsequent prompts from IPC clients until idle timeout.
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
    no_wait: bool,
) -> Result<i32> {
    let mut renderer = make_renderer(output_format);

    // Default TTL for the queue owner (5 minutes).
    let queue_ttl_secs: u64 = 300;

    // Find or create session record.
    // If an existing open session is found, reuse its key so conversation
    // history accumulates across multiple runs.
    let resolved_dir = find_git_root(&cwd).unwrap_or_else(|| cwd.clone());
    let dir_str = resolved_dir.to_string_lossy();
    let sess_name = session_name.as_deref().unwrap_or("");
    let key = session_key(agent_name, &dir_str, sess_name);

    let sess_file = session_dir().join(format!("{key}.json"));
    let existing = SessionRecord::load(&sess_file).ok().flatten();
    let is_resume = existing.as_ref().is_some_and(|r| !r.closed);

    if existing.is_none() {
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
            acp_session_id: None,
        };
        if let Err(e) = record.save(&sess_file) {
            renderer.error(&format!("failed to save session: {e}"));
        }
    }

    if is_resume {
        renderer.session_info(&format!("resuming session {}", &key[..12.min(key.len())]));
    }

    // Check if a queue owner already exists for this session.
    // Use the lease file (with heartbeat + PID liveness check) for reliable
    // ownership detection. If a valid lease exists, connect as a queue client
    // instead of starting a new bridge.
    if let Some(lease) = LeaseFile::read(&key)
        && lease.is_valid(queue_ttl_secs)
    {
        match QueueClient::connect(&key).await {
            Ok(mut client) => {
                renderer.session_info("Connected to queue owner");

                // --no-wait: enqueue the prompt and return immediately.
                if no_wait {
                    let position = client.enqueue_only(vec![prompt_text.clone()]).await?;
                    renderer.session_info(&format!("Prompt queued (position {position})"));
                    renderer.done();

                    // Log the user prompt (best-effort). No assistant entry since
                    // we won't wait for the response.
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let user_entry = ConversationEntry {
                        role: "user".to_string(),
                        content: prompt_text,
                        timestamp: now,
                    };
                    let _ = append_entry(&key, &user_entry);

                    return Ok(0);
                }

                let result = client
                    .prompt(vec![prompt_text.clone()], &mut *renderer, &permission_mode)
                    .await;
                renderer.done();

                // Log conversation history (best-effort).
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let user_entry = ConversationEntry {
                    role: "user".to_string(),
                    content: prompt_text,
                    timestamp: now,
                };
                let _ = append_entry(&key, &user_entry);

                if let Ok(ref pr) = result
                    && !pr.content.is_empty()
                {
                    let assistant_entry = ConversationEntry {
                        role: "assistant".to_string(),
                        content: pr.content.clone(),
                        timestamp: now,
                    };
                    let _ = append_entry(&key, &assistant_entry);
                }

                return result.map(|_| 0);
            }
            Err(e) => {
                // Socket connection failed despite valid lease.
                if no_wait {
                    return Err(AcpCliError::Usage(
                        "No active session. Run without --no-wait first to start a session."
                            .to_string(),
                    ));
                }
                // Fall through to become a new queue owner (owner may have
                // crashed after writing the lease).
                renderer.session_info(&format!(
                    "Could not connect to queue owner (pid {}): {e}; starting new session",
                    lease.pid
                ));
            }
        }
    }

    // --- Become the queue owner ---

    // --no-wait requires an existing queue owner to accept the prompt. Since no
    // valid lease was found, we cannot fire-and-forget.
    if no_wait {
        return Err(AcpCliError::Usage(
            "No active session. Run without --no-wait first to start a session.".to_string(),
        ));
    }

    // Write PID file so `cancel` and `status` commands can find us.
    // The guard removes it automatically when this function returns.
    let _pid_guard = PidGuard::new(&key).map_err(|e| {
        renderer.error(&format!("failed to write pid file: {e}"));
        AcpCliError::Io(e)
    })?;

    // Write lease file to claim queue ownership.
    LeaseFile::write(&key).map_err(|e| {
        renderer.error(&format!("failed to write lease file: {e}"));
        AcpCliError::Io(e)
    })?;

    // Start IPC server so future clients can connect.
    let listener = start_ipc_server(&key).await.map_err(|e| {
        LeaseFile::remove(&key);
        renderer.error(&format!("failed to start IPC server: {e}"));
        AcpCliError::Io(e)
    })?;

    // Start bridge
    let mut bridge = AcpBridge::start(command, args, cwd).await?;
    let cancel = bridge.cancel_handle();

    // Send the first prompt (get oneshot receiver without blocking)
    let prompt_reply = bridge.send_prompt(vec![prompt_text.clone()]).await?;

    // Run event loop for the first prompt
    let loop_result = event_loop(
        &mut bridge.evt_rx,
        prompt_reply,
        &cancel,
        &mut renderer,
        &permission_mode,
        timeout_secs,
    )
    .await;

    // Update the session record with the new ACP session ID (best-effort).
    if let Ok(ref res) = loop_result
        && let Some(ref new_acp_id) = res.acp_session_id
        && let Ok(Some(mut record)) = SessionRecord::load(&sess_file)
    {
        let _ = record.update_acp_session_id(new_acp_id.clone(), &sess_file);
    }

    // Log conversation history (best-effort, don't fail the prompt on log errors)
    if let Ok(ref res) = loop_result {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let user_entry = ConversationEntry {
            role: "user".to_string(),
            content: prompt_text,
            timestamp: now,
        };
        let _ = append_entry(&key, &user_entry);

        if !res.assistant_text.is_empty() {
            let assistant_entry = ConversationEntry {
                role: "assistant".to_string(),
                content: res.assistant_text.clone(),
                timestamp: now,
            };
            let _ = append_entry(&key, &assistant_entry);
        }
    }

    // If the first prompt failed, clean up and return early.
    if loop_result.is_err() {
        LeaseFile::remove(&key);
        crate::queue::ipc::cleanup_socket(&key);
        let _ = bridge.shutdown().await;
        return loop_result.map(|r| r.exit_code);
    }

    let first_exit_code = loop_result.map(|r| r.exit_code)?;

    // First prompt succeeded — enter queue owner mode to serve subsequent
    // prompts from IPC clients until idle timeout.
    let owner = QueueOwner::new(bridge, listener, &key, queue_ttl_secs).await?;
    let _ = owner.run().await;

    Ok(first_exit_code)
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
    result.map(|r| r.exit_code)
}
