use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::session::scoping::session_dir;

/// Return the Unix socket path for a given session key.
/// Uses first 12 hex chars of the key to stay within macOS SUN_LEN limit (~104 bytes).
pub fn socket_path(session_key: &str) -> PathBuf {
    let short_key = &session_key[..session_key.len().min(12)];
    session_dir().join(format!("{short_key}.sock"))
}

/// Start an IPC server on a Unix socket.
///
/// Removes any stale socket file before binding. The caller is responsible for
/// accepting connections on the returned listener.
pub async fn start_ipc_server(session_key: &str) -> std::io::Result<UnixListener> {
    let path = socket_path(session_key);
    // Ensure the parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Remove stale socket file if it exists.
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path)
}

/// Connect to an existing IPC server for the given session.
pub async fn connect_ipc(session_key: &str) -> std::io::Result<UnixStream> {
    let path = socket_path(session_key);
    UnixStream::connect(&path).await
}

/// Send a message over a Unix stream as a single JSON line.
pub async fn send_message<T: Serialize>(stream: &mut UnixStream, msg: &T) -> std::io::Result<()> {
    let json = serde_json::to_string(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    stream.write_all(json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await
}

/// Read one message from a Unix stream.
///
/// Returns `Ok(None)` when the stream is closed (EOF).
pub async fn recv_message<T: DeserializeOwned>(
    reader: &mut BufReader<UnixStream>,
) -> std::io::Result<Option<T>> {
    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(None);
    }
    let msg = serde_json::from_str(&line)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(msg))
}

/// Remove the socket file for a session (best-effort cleanup).
pub fn cleanup_socket(session_key: &str) {
    let _ = std::fs::remove_file(socket_path(session_key));
}
