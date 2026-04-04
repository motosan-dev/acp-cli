use tokio::io::BufReader;
use tokio::net::UnixStream;

use crate::bridge::events::PromptResult;
use crate::client::permissions::PermissionMode;
use crate::error::{AcpCliError, Result};
use crate::output::OutputRenderer;

use super::ipc::{connect_ipc, recv_message, send_message};
use super::messages::{QueueRequest, QueueResponse};

/// A client that connects to an existing queue owner via Unix socket and
/// enqueues prompts for processing.
pub struct QueueClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl QueueClient {
    /// Connect to the queue owner's Unix socket for the given session key.
    pub async fn connect(session_key: &str) -> std::io::Result<Self> {
        let stream = connect_ipc(session_key).await?;
        // Split the stream: we need an owned reader half for BufReader and keep
        // the original stream for writing. Since Unix sockets are full-duplex,
        // we clone the underlying fd via `into_std` + `try_clone` + `from_std`.
        let std_stream = stream.into_std()?;
        let reader_std = std_stream.try_clone()?;
        let write_stream = UnixStream::from_std(std_stream)?;
        let read_stream = UnixStream::from_std(reader_std)?;

        Ok(Self {
            stream: write_stream,
            reader: BufReader::new(read_stream),
        })
    }

    /// Send a prompt to the queue owner and stream response events back
    /// through the renderer. Returns the final `PromptResult` when the owner
    /// finishes processing.
    pub async fn prompt(
        &mut self,
        messages: Vec<String>,
        renderer: &mut dyn OutputRenderer,
        _permission_mode: &PermissionMode,
    ) -> Result<PromptResult> {
        // Generate a unique reply ID for correlation.
        let reply_id = generate_reply_id();

        // Send the prompt request to the queue owner.
        let request = QueueRequest::Prompt {
            messages,
            reply_id: reply_id.clone(),
        };
        send_message(&mut self.stream, &request)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to send prompt: {e}")))?;

        // Loop receiving responses until we get a PromptResult or Error.
        loop {
            let response: Option<QueueResponse> = recv_message(&mut self.reader)
                .await
                .map_err(|e| AcpCliError::Connection(format!("failed to read response: {e}")))?;

            match response {
                Some(QueueResponse::Queued {
                    position,
                    reply_id: _,
                }) => {
                    renderer.session_info(&format!("Queued at position {position}"));
                }
                Some(QueueResponse::Event { kind, data }) => match kind.as_str() {
                    "text_chunk" => renderer.text_chunk(&data),
                    "tool_use" => renderer.tool_status(&data),
                    "tool_result" => {
                        if let Some((name, output)) = data.split_once('\x00') {
                            renderer.tool_result(name, output);
                        }
                    }
                    _ => {
                        // Unknown event kind — log as info and continue.
                        renderer.session_info(&format!("event({kind}): {data}"));
                    }
                },
                Some(QueueResponse::PromptResult {
                    content,
                    stop_reason,
                    reply_id: _,
                }) => {
                    return Ok(PromptResult {
                        content,
                        stop_reason,
                    });
                }
                Some(QueueResponse::StatusResponse { state, queue_depth }) => {
                    renderer.session_info(&format!("status: {state}, depth: {queue_depth}"));
                }
                Some(QueueResponse::Error { message }) => {
                    return Err(AcpCliError::Agent(message));
                }
                Some(QueueResponse::Ok) => {
                    // Acknowledgement for non-prompt commands; skip.
                }
                None => {
                    // Stream closed unexpectedly.
                    return Err(AcpCliError::Connection(
                        "queue owner disconnected".to_string(),
                    ));
                }
            }
        }
    }

    /// Send a prompt and wait only for the `Queued` acknowledgement, returning
    /// the queue position. This is used by `--no-wait` mode so the CLI can exit
    /// immediately after confirming the prompt was accepted.
    pub async fn enqueue_only(&mut self, messages: Vec<String>) -> Result<usize> {
        let reply_id = generate_reply_id();

        let request = QueueRequest::Prompt {
            messages,
            reply_id: reply_id.clone(),
        };
        send_message(&mut self.stream, &request)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to send prompt: {e}")))?;

        // Wait only for the Queued response, then return immediately.
        loop {
            let response: Option<QueueResponse> = recv_message(&mut self.reader)
                .await
                .map_err(|e| AcpCliError::Connection(format!("failed to read response: {e}")))?;

            match response {
                Some(QueueResponse::Queued { position, .. }) => {
                    return Ok(position);
                }
                Some(QueueResponse::Error { message }) => {
                    return Err(AcpCliError::Agent(message));
                }
                None => {
                    return Err(AcpCliError::Connection(
                        "queue owner disconnected before acknowledging prompt".to_string(),
                    ));
                }
                // Skip any other messages that arrive before the Queued ack.
                _ => continue,
            }
        }
    }

    /// Send a set-mode request to the queue owner.
    pub async fn set_mode(&mut self, mode: &str) -> Result<()> {
        let request = QueueRequest::SetMode {
            mode: mode.to_string(),
        };
        send_message(&mut self.stream, &request)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to send set-mode: {e}")))?;

        let response: Option<QueueResponse> = recv_message(&mut self.reader)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to read response: {e}")))?;

        match response {
            Some(QueueResponse::Ok) => {
                println!("Mode set to: {mode}");
                Ok(())
            }
            Some(QueueResponse::Error { message }) => Err(AcpCliError::Agent(message)),
            _ => Err(AcpCliError::Connection(
                "unexpected response to set-mode request".to_string(),
            )),
        }
    }

    /// Send a set-config request to the queue owner.
    pub async fn set_config(&mut self, key: &str, value: &str) -> Result<()> {
        let request = QueueRequest::SetConfig {
            key: key.to_string(),
            value: value.to_string(),
        };
        send_message(&mut self.stream, &request)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to send set-config: {e}")))?;

        let response: Option<QueueResponse> = recv_message(&mut self.reader)
            .await
            .map_err(|e| AcpCliError::Connection(format!("failed to read response: {e}")))?;

        match response {
            Some(QueueResponse::Ok) => {
                println!("Config set: {key} = {value}");
                Ok(())
            }
            Some(QueueResponse::Error { message }) => Err(AcpCliError::Agent(message)),
            _ => Err(AcpCliError::Connection(
                "unexpected response to set-config request".to_string(),
            )),
        }
    }

    /// Send a cancel request to the queue owner.
    pub async fn cancel(&mut self) -> std::io::Result<()> {
        send_message(&mut self.stream, &QueueRequest::Cancel).await
    }

    /// Request status from the queue owner.
    pub async fn status(&mut self) -> std::io::Result<String> {
        send_message(&mut self.stream, &QueueRequest::Status).await?;

        let response: Option<QueueResponse> = recv_message(&mut self.reader).await?;
        match response {
            Some(QueueResponse::StatusResponse { state, queue_depth }) => {
                Ok(format!("state: {state}, queue_depth: {queue_depth}"))
            }
            Some(QueueResponse::Error { message }) => {
                Err(std::io::Error::other(format!("status error: {message}")))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected response to status request",
            )),
        }
    }
}

/// Generate a random reply ID using timestamp and random bytes.
///
/// Uses a simple hex-encoded format without requiring the `uuid` crate.
fn generate_reply_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Mix timestamp with process id and a simple counter for uniqueness.
    let pid = std::process::id();
    format!("{timestamp:x}-{pid:x}")
}
