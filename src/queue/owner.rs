use std::collections::VecDeque;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::UnixListener;
use tokio::net::UnixStream;

use crate::bridge::AcpBridge;
use crate::bridge::events::BridgeEvent;
use crate::error::Result;
use crate::queue::ipc::{recv_message, send_message};
use crate::queue::lease::LeaseFile;
use crate::queue::messages::{QueueRequest, QueueResponse};

/// A queued prompt waiting to be executed.
struct PendingPrompt {
    messages: Vec<String>,
    reply_id: String,
    client: UnixStream,
}

/// The queue owner process — owns the ACP bridge and multiplexes incoming
/// prompts from IPC clients, executing them sequentially.
pub struct QueueOwner {
    bridge: AcpBridge,
    listener: UnixListener,
    session_key: String,
    ttl_secs: u64,
}

impl QueueOwner {
    /// Create a new queue owner.
    ///
    /// The caller must have already written the lease file and started the IPC
    /// server (`UnixListener`).
    pub async fn new(
        bridge: AcpBridge,
        listener: UnixListener,
        session_key: &str,
        ttl_secs: u64,
    ) -> Result<Self> {
        Ok(Self {
            bridge,
            listener,
            session_key: session_key.to_string(),
            ttl_secs,
        })
    }

    /// Run the owner event loop.
    ///
    /// Uses `tokio::select!` to concurrently handle:
    /// 1. New client connections from the Unix socket.
    /// 2. Bridge events (forwarded to the active prompt's client).
    /// 3. Heartbeat timer (updates lease file every 5 seconds).
    /// 4. Idle timeout (shuts down after `ttl_secs` with an empty queue).
    pub async fn run(mut self) -> Result<()> {
        let mut queue: VecDeque<PendingPrompt> = VecDeque::new();
        let mut active_client: Option<UnixStream> = None;
        let mut active_reply_id: Option<String> = None;
        let mut prompt_reply: Option<
            tokio::sync::oneshot::Receiver<Result<crate::bridge::events::PromptResult>>,
        > = None;

        let heartbeat_interval = Duration::from_secs(5);
        let mut heartbeat_timer = tokio::time::interval(heartbeat_interval);
        heartbeat_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the first immediate tick.
        heartbeat_timer.tick().await;

        let idle_duration = Duration::from_secs(self.ttl_secs);
        let idle_deadline = tokio::time::sleep(idle_duration);
        tokio::pin!(idle_deadline);

        loop {
            tokio::select! {
                // --- 1. Accept new client connections ---
                accept_result = self.listener.accept() => {
                    match accept_result {
                        Ok((stream, _addr)) => {
                            // Reset idle timer on any connection.
                            idle_deadline.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                            self.handle_client(stream, &mut queue, &mut active_client, &mut active_reply_id, &mut prompt_reply).await;
                        }
                        Err(e) => {
                            eprintln!("[queue-owner] accept error: {e}");
                        }
                    }
                }

                // --- 2. Bridge events (forward to active client) ---
                event = self.bridge.evt_rx.recv() => {
                    match event {
                        Some(evt) => {
                            self.forward_event(&evt, &mut active_client).await;
                        }
                        None => {
                            // Bridge channel closed — agent died.
                            eprintln!("[queue-owner] bridge closed, shutting down");
                            break;
                        }
                    }
                }

                // --- 3. Prompt completion ---
                result = async {
                    match prompt_reply.as_mut() {
                        Some(rx) => rx.await,
                        None => std::future::pending().await,
                    }
                } => {
                    prompt_reply = None;
                    let (content, stop_reason) = match result {
                        Ok(Ok(pr)) => (pr.content, pr.stop_reason),
                        Ok(Err(e)) => {
                            if let Some(client) = active_client.as_mut() {
                                let _ = send_message(client, &QueueResponse::Error {
                                    message: e.to_string(),
                                }).await;
                            }
                            (String::new(), "error".to_string())
                        }
                        Err(_) => {
                            if let Some(client) = active_client.as_mut() {
                                let _ = send_message(client, &QueueResponse::Error {
                                    message: "bridge reply dropped".to_string(),
                                }).await;
                            }
                            (String::new(), "error".to_string())
                        }
                    };

                    // Send PromptResult to the active client.
                    if let Some(client) = active_client.as_mut() {
                        let reply_id = active_reply_id.take().unwrap_or_default();
                        let _ = send_message(client, &QueueResponse::PromptResult {
                            reply_id,
                            content,
                            stop_reason,
                        }).await;
                    }
                    active_client = None;
                    active_reply_id = None;

                    // Process the next prompt in the queue.
                    self.dispatch_next(&mut queue, &mut active_client, &mut active_reply_id, &mut prompt_reply).await;

                    // If queue is empty after dispatch, reset idle timer.
                    if queue.is_empty() && active_client.is_none() {
                        idle_deadline.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                    }
                }

                // --- 4. Heartbeat timer ---
                _ = heartbeat_timer.tick() => {
                    if let Err(e) = LeaseFile::update_heartbeat(&self.session_key) {
                        eprintln!("[queue-owner] heartbeat error: {e}");
                    }
                }

                // --- 5. Idle timeout ---
                _ = &mut idle_deadline => {
                    if queue.is_empty() && active_client.is_none() {
                        eprintln!("[queue-owner] idle timeout ({} s), shutting down", self.ttl_secs);
                        break;
                    }
                    // Still busy — reset and try again later.
                    idle_deadline.as_mut().reset(tokio::time::Instant::now() + idle_duration);
                }
            }
        }

        // Cleanup
        self.shutdown().await;
        Ok(())
    }

    /// Handle a newly connected client — read one request and act on it.
    async fn handle_client(
        &self,
        stream: UnixStream,
        queue: &mut VecDeque<PendingPrompt>,
        active_client: &mut Option<UnixStream>,
        active_reply_id: &mut Option<String>,
        prompt_reply: &mut Option<
            tokio::sync::oneshot::Receiver<Result<crate::bridge::events::PromptResult>>,
        >,
    ) {
        // We need to read a request from the stream. Clone the stream fd for
        // the BufReader while keeping the original for sending responses.
        let std_stream = match stream.into_std() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[queue-owner] failed to convert stream: {e}");
                return;
            }
        };
        let read_std = match std_stream.try_clone() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[queue-owner] failed to clone stream: {e}");
                return;
            }
        };
        let mut write_stream = match UnixStream::from_std(std_stream) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[queue-owner] failed to convert write stream: {e}");
                return;
            }
        };
        let read_stream = match UnixStream::from_std(read_std) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[queue-owner] failed to convert read stream: {e}");
                return;
            }
        };

        let mut reader = BufReader::new(read_stream);
        let request: Option<QueueRequest> = match recv_message(&mut reader).await {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[queue-owner] failed to read request: {e}");
                return;
            }
        };

        match request {
            Some(QueueRequest::Prompt { messages, reply_id }) => {
                if active_client.is_some() {
                    // Already processing a prompt — enqueue.
                    let position = queue.len() + 1;
                    let _ = send_message(
                        &mut write_stream,
                        &QueueResponse::Queued {
                            reply_id: reply_id.clone(),
                            position,
                        },
                    )
                    .await;
                    queue.push_back(PendingPrompt {
                        messages,
                        reply_id,
                        client: write_stream,
                    });
                } else {
                    // No active prompt — execute immediately.
                    self.start_prompt(
                        messages,
                        &reply_id,
                        write_stream,
                        active_client,
                        active_reply_id,
                        prompt_reply,
                    )
                    .await;
                }
            }
            Some(QueueRequest::Cancel) => {
                let _ = self.bridge.cancel().await;
                let _ = send_message(
                    &mut write_stream,
                    &QueueResponse::StatusResponse {
                        state: "cancel_requested".to_string(),
                        queue_depth: queue.len(),
                    },
                )
                .await;
            }
            Some(QueueRequest::Status) => {
                let state = if active_client.is_some() {
                    "busy"
                } else {
                    "idle"
                };
                let _ = send_message(
                    &mut write_stream,
                    &QueueResponse::StatusResponse {
                        state: state.to_string(),
                        queue_depth: queue.len(),
                    },
                )
                .await;
            }
            Some(QueueRequest::SetMode { mode }) => match self.bridge.set_mode(mode).await {
                Ok(()) => {
                    let _ = send_message(&mut write_stream, &QueueResponse::Ok).await;
                }
                Err(e) => {
                    let _ = send_message(
                        &mut write_stream,
                        &QueueResponse::Error {
                            message: e.to_string(),
                        },
                    )
                    .await;
                }
            },
            Some(QueueRequest::SetConfig { key, value }) => {
                match self.bridge.set_config(key, value).await {
                    Ok(()) => {
                        let _ = send_message(&mut write_stream, &QueueResponse::Ok).await;
                    }
                    Err(e) => {
                        let _ = send_message(
                            &mut write_stream,
                            &QueueResponse::Error {
                                message: e.to_string(),
                            },
                        )
                        .await;
                    }
                }
            }
            None => {
                // Client disconnected before sending a request.
            }
        }
    }

    /// Start executing a prompt on the bridge.
    async fn start_prompt(
        &self,
        messages: Vec<String>,
        reply_id: &str,
        client: UnixStream,
        active_client: &mut Option<UnixStream>,
        active_reply_id: &mut Option<String>,
        prompt_reply: &mut Option<
            tokio::sync::oneshot::Receiver<Result<crate::bridge::events::PromptResult>>,
        >,
    ) {
        match self.bridge.send_prompt(messages).await {
            Ok(rx) => {
                *active_client = Some(client);
                *active_reply_id = Some(reply_id.to_string());
                *prompt_reply = Some(rx);
            }
            Err(e) => {
                let mut c = client;
                let _ = send_message(
                    &mut c,
                    &QueueResponse::Error {
                        message: e.to_string(),
                    },
                )
                .await;
            }
        }
    }

    /// Dispatch the next queued prompt (if any).
    async fn dispatch_next(
        &self,
        queue: &mut VecDeque<PendingPrompt>,
        active_client: &mut Option<UnixStream>,
        active_reply_id: &mut Option<String>,
        prompt_reply: &mut Option<
            tokio::sync::oneshot::Receiver<Result<crate::bridge::events::PromptResult>>,
        >,
    ) {
        if let Some(pending) = queue.pop_front() {
            self.start_prompt(
                pending.messages,
                &pending.reply_id,
                pending.client,
                active_client,
                active_reply_id,
                prompt_reply,
            )
            .await;
        }
    }

    /// Forward a bridge event to the active client as a `QueueResponse::Event`.
    async fn forward_event(&self, event: &BridgeEvent, active_client: &mut Option<UnixStream>) {
        let response = match event {
            BridgeEvent::TextChunk { text } => Some(QueueResponse::Event {
                kind: "text_chunk".to_string(),
                data: text.clone(),
            }),
            BridgeEvent::ToolUse { name } => Some(QueueResponse::Event {
                kind: "tool_use".to_string(),
                data: name.clone(),
            }),
            BridgeEvent::PromptDone { stop_reason } => Some(QueueResponse::Event {
                kind: "prompt_done".to_string(),
                data: stop_reason.clone(),
            }),
            BridgeEvent::Error { message } => Some(QueueResponse::Event {
                kind: "error".to_string(),
                data: message.clone(),
            }),
            BridgeEvent::SessionCreated { session_id } => Some(QueueResponse::Event {
                kind: "session_created".to_string(),
                data: session_id.clone(),
            }),
            BridgeEvent::AgentExited { code } => Some(QueueResponse::Event {
                kind: "agent_exited".to_string(),
                data: code.map(|c| c.to_string()).unwrap_or_default(),
            }),
            BridgeEvent::PermissionRequest { .. } => {
                // Permission requests are handled by the owner process itself,
                // not forwarded to IPC clients.
                None
            }
        };

        if let Some(resp) = response
            && let Some(client) = active_client.as_mut()
        {
            let _ = send_message(client, &resp).await;
        }
    }

    /// Shut down the owner: clean up lease, socket, and bridge.
    async fn shutdown(self) {
        LeaseFile::remove(&self.session_key);
        crate::queue::ipc::cleanup_socket(&self.session_key);
        let _ = self.bridge.shutdown().await;
    }
}
