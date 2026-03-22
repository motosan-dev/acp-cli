pub mod commands;
pub mod events;

pub use commands::BridgeCommand;
pub use events::{
    BridgeEvent, PermissionKind, PermissionOption, PermissionOutcome, PromptResult, ToolCallInfo,
};

use std::path::PathBuf;

use agent_client_protocol::{self as acp, Agent as _};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::client::BridgedAcpClient;
use crate::error::{AcpCliError, Result};

/// Core bridge that manages the ACP agent process lifecycle.
///
/// Spawns the agent in a dedicated blocking thread with a `LocalSet` (required
/// because ACP futures are `!Send`), and exposes an async command/event API
/// for the main thread to drive prompts and receive streamed output.
pub struct AcpBridge {
    cmd_tx: mpsc::Sender<BridgeCommand>,
    pub evt_rx: mpsc::UnboundedReceiver<BridgeEvent>,
    handle: JoinHandle<std::result::Result<(), AcpCliError>>,
}

/// Handle used to send cancel/shutdown commands independently of `evt_rx`.
///
/// Obtained via `AcpBridge::cancel_handle()` so the caller can borrow `evt_rx`
/// mutably while still being able to send cancel commands.
#[derive(Clone)]
pub struct BridgeCancelHandle {
    cmd_tx: mpsc::Sender<BridgeCommand>,
}

impl BridgeCancelHandle {
    /// Request cancellation of the current prompt (best-effort).
    pub async fn cancel(&self) -> Result<()> {
        let _ = self.cmd_tx.send(BridgeCommand::Cancel).await;
        Ok(())
    }
}

impl AcpBridge {
    /// Start the ACP bridge by spawning the agent process in a background thread.
    ///
    /// The agent is launched via `command` with the given `args`, and the working
    /// directory for the ACP session is set to `cwd`.
    pub async fn start(command: String, args: Vec<String>, cwd: PathBuf) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<BridgeCommand>(16);
        let (evt_tx, evt_rx) = mpsc::unbounded_channel::<BridgeEvent>();

        let handle = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| AcpCliError::Connection(format!("runtime: {e}")))?;
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, acp_thread_main(cmd_rx, evt_tx, command, args, cwd))
        });

        Ok(Self {
            cmd_tx,
            evt_rx,
            handle,
        })
    }

    /// Obtain a lightweight cancel handle that can be used while `evt_rx` is
    /// borrowed mutably.
    pub fn cancel_handle(&self) -> BridgeCancelHandle {
        BridgeCancelHandle {
            cmd_tx: self.cmd_tx.clone(),
        }
    }

    /// Send a prompt to the agent and wait for the result.
    ///
    /// Text content is streamed in real-time via `BridgeEvent::TextChunk` events
    /// on `evt_rx`. The returned `PromptResult.content` will be empty because
    /// the main thread is expected to collect content from those events.
    pub async fn prompt(&self, messages: Vec<String>) -> Result<PromptResult> {
        let reply_rx = self.send_prompt(messages).await?;
        reply_rx
            .await
            .map_err(|_| AcpCliError::Connection("bridge reply dropped".into()))?
    }

    /// Send a prompt command without awaiting the reply.
    ///
    /// Returns a oneshot receiver that will resolve when the prompt completes.
    /// This allows the caller to drive `evt_rx` concurrently with waiting for
    /// the prompt result, avoiding borrow conflicts on `self`.
    pub async fn send_prompt(
        &self,
        messages: Vec<String>,
    ) -> Result<tokio::sync::oneshot::Receiver<Result<PromptResult>>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(BridgeCommand::Prompt {
                messages,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpCliError::Connection("bridge channel closed".into()))?;
        Ok(reply_rx)
    }

    /// Request cancellation of the current prompt (best-effort).
    pub async fn cancel(&self) -> Result<()> {
        let _ = self.cmd_tx.send(BridgeCommand::Cancel).await;
        Ok(())
    }

    /// Set the session mode on the ACP connection.
    pub async fn set_mode(&self, mode: String) -> Result<()> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(BridgeCommand::SetMode {
                mode,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpCliError::Connection("bridge channel closed".into()))?;
        reply_rx
            .await
            .map_err(|_| AcpCliError::Connection("bridge reply dropped".into()))?
    }

    /// Set a session config option on the ACP connection.
    pub async fn set_config(&self, key: String, value: String) -> Result<()> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(BridgeCommand::SetConfig {
                key,
                value,
                reply: reply_tx,
            })
            .await
            .map_err(|_| AcpCliError::Connection("bridge channel closed".into()))?;
        reply_rx
            .await
            .map_err(|_| AcpCliError::Connection("bridge reply dropped".into()))?
    }

    /// Gracefully shut down the bridge, killing the agent process and joining
    /// the background thread.
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.cmd_tx.send(BridgeCommand::Shutdown).await;
        match self.handle.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(AcpCliError::Connection(format!("join: {e}"))),
        }
    }
}

/// Main loop running inside `spawn_blocking` + `LocalSet`.
///
/// Spawns the agent child process, establishes the ACP connection, initializes
/// the protocol, creates a session, then enters a command loop that processes
/// `BridgeCommand` messages from the main thread.
async fn acp_thread_main(
    mut cmd_rx: mpsc::Receiver<BridgeCommand>,
    evt_tx: mpsc::UnboundedSender<BridgeEvent>,
    command: String,
    args: Vec<String>,
    cwd: PathBuf,
) -> Result<()> {
    // 1. Spawn agent process
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut cmd = tokio::process::Command::new(&command);
    cmd.args(&args_refs)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true);

    // Remove ANTHROPIC_API_KEY to prevent OAuth setup tokens (sk-ant-oat01-*)
    // from being misidentified as API keys by the Claude Agent SDK.
    cmd.env_remove("ANTHROPIC_API_KEY");

    // Actively inject the OAuth token so claude-agent-acp doesn't have to
    // resolve it from ~/.claude.json or Keychain (which can be stale).
    if let Some(token) = resolve_claude_auth_token() {
        cmd.env("ANTHROPIC_AUTH_TOKEN", &token);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AcpCliError::Agent(format!("{command}: {e}")))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AcpCliError::Agent("no stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AcpCliError::Agent("no stdout".into()))?;

    let client = BridgedAcpClient {
        evt_tx: evt_tx.clone(),
    };

    let (conn, handle_io) =
        acp::ClientSideConnection::new(client, stdin.compat_write(), stdout.compat(), |fut| {
            tokio::task::spawn_local(fut);
        });

    // Drive the I/O loop in the background on the local task set.
    tokio::task::spawn_local(async move {
        if let Err(e) = handle_io.await {
            eprintln!("[acp-cli] I/O error: {e}");
        }
    });

    // 2. Initialize
    conn.initialize(
        acp::InitializeRequest::new(acp::ProtocolVersion::V1).client_info(
            acp::Implementation::new("acp-cli", env!("CARGO_PKG_VERSION")),
        ),
    )
    .await
    .map_err(|e| AcpCliError::Connection(format!("initialize: {e}")))?;

    // 3. Create session
    let session = conn
        .new_session(acp::NewSessionRequest::new(cwd))
        .await
        .map_err(|e| AcpCliError::Connection(format!("new_session: {e}")))?;

    let session_id = session.session_id;
    let _ = evt_tx.send(BridgeEvent::SessionCreated {
        session_id: session_id.0.to_string(),
    });

    // 4. Command loop
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            BridgeCommand::Prompt { messages, reply } => {
                let content_blocks: Vec<acp::ContentBlock> =
                    messages.into_iter().map(|m| m.into()).collect();
                let result = conn
                    .prompt(acp::PromptRequest::new(session_id.clone(), content_blocks))
                    .await;
                match result {
                    Ok(response) => {
                        let stop_reason = serde_json::to_value(response.stop_reason)
                            .ok()
                            .and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| "unknown".to_string());
                        let _ = evt_tx.send(BridgeEvent::PromptDone {
                            stop_reason: stop_reason.clone(),
                        });
                        // Content was already streamed via session_notification -> TextChunk.
                        // The main thread collects content from BridgeEvent::TextChunk events.
                        let _ = reply.send(Ok(PromptResult {
                            content: String::new(),
                            stop_reason,
                        }));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpCliError::Agent(format!("{e}"))));
                    }
                }
            }
            BridgeCommand::Cancel => {
                // ACP cancel not yet implemented in SDK
            }
            BridgeCommand::SetMode { mode, reply } => {
                let mode_id = acp::SessionModeId::new(mode);
                let request = acp::SetSessionModeRequest::new(session_id.clone(), mode_id);
                match conn.set_session_mode(request).await {
                    Ok(_) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ =
                            reply.send(Err(AcpCliError::Agent(format!("set_session_mode: {e}"))));
                    }
                }
            }
            BridgeCommand::SetConfig { key, value, reply } => {
                let config_id = acp::SessionConfigId::new(key);
                let value_id = acp::SessionConfigValueId::new(value);
                let request = acp::SetSessionConfigOptionRequest::new(
                    session_id.clone(),
                    config_id,
                    value_id,
                );
                match conn.set_session_config_option(request).await {
                    Ok(_) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpCliError::Agent(format!(
                            "set_session_config_option: {e}"
                        ))));
                    }
                }
            }
            BridgeCommand::Shutdown => break,
        }
    }

    // Cleanup
    child.kill().await.ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// Claude auth token resolution
// ---------------------------------------------------------------------------

/// Resolve an OAuth token for claude-agent-acp, checking (in order):
/// 1. `ANTHROPIC_AUTH_TOKEN` env var (already set externally)
/// 2. `~/.claude.json` → `oauthAccount.accessToken`
/// 3. macOS Keychain (`security find-generic-password`)
fn resolve_claude_auth_token() -> Option<String> {
    // 1. Already set externally
    if let Ok(t) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }

    // 2. ~/.claude.json
    if let Some(token) = read_claude_json_token() {
        return Some(token);
    }

    // 3. macOS Keychain
    #[cfg(target_os = "macos")]
    if let Some(token) = read_keychain_token() {
        return Some(token);
    }

    None
}

/// Read the OAuth access token from `~/.claude.json`.
fn read_claude_json_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".claude.json");
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    json.pointer("/oauthAccount/accessToken")
        .or_else(|| json.get("accessToken"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Read the Claude Code OAuth token from the macOS Keychain.
#[cfg(target_os = "macos")]
fn read_keychain_token() -> Option<String> {
    // Try known service names used by Claude Code
    for service in &["Claude Code", "claude.ai", "anthropic.claude"] {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-s", service, "-w"])
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if output.status.success() {
            let token = String::from_utf8(output.stdout)
                .ok()?
                .trim()
                .to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}
