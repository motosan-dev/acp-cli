use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AcpCliError, Result};
use crate::session::history::load_history;
use crate::session::persistence::SessionRecord;
use crate::session::pid;
use crate::session::scoping::{find_git_root, session_dir, session_key};

/// Create a new session record and persist it to disk.
pub fn sessions_new(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let record = SessionRecord {
        id: key.clone(),
        agent: agent.to_string(),
        cwd: resolved_dir.clone(),
        name: name.map(|s| s.to_string()),
        created_at: now,
        closed: false,
        acp_session_id: None,
    };

    let path = session_dir().join(format!("{key}.json"));
    record.save(&path).map_err(AcpCliError::Io)?;

    println!("Session created:");
    println!("  id:    {key}");
    println!("  agent: {agent}");
    println!("  cwd:   {}", resolved_dir.display());
    if let Some(n) = name {
        println!("  name:  {n}");
    }
    println!("  file:  {}", path.display());

    Ok(())
}

/// List all session files, optionally filtered by agent and cwd.
pub fn sessions_list(agent: Option<&str>, cwd: Option<&str>) -> Result<()> {
    let dir = session_dir();
    if !dir.exists() {
        println!("No sessions found.");
        return Ok(());
    }

    let entries = std::fs::read_dir(&dir).map_err(AcpCliError::Io)?;
    let mut found = false;

    for entry in entries {
        let entry = entry.map_err(AcpCliError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let record = match SessionRecord::load(&path).map_err(AcpCliError::Io)? {
            Some(r) => r,
            None => continue,
        };

        // Filter by agent if specified
        if let Some(a) = agent
            && record.agent != a
        {
            continue;
        }

        // Filter by cwd if specified
        if let Some(c) = cwd {
            let cwd_path = Path::new(c);
            let resolved = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
            if record.cwd != resolved {
                continue;
            }
        }

        if !found {
            println!("{:<12} {:<10} {:<6} CWD", "ID (short)", "AGENT", "STATUS");
            println!("{}", "-".repeat(72));
            found = true;
        }

        let short_id = &record.id[..12.min(record.id.len())];
        let status = if record.closed { "closed" } else { "open" };
        println!(
            "{:<12} {:<10} {:<6} {}",
            short_id,
            record.agent,
            status,
            record.cwd.display()
        );
    }

    if !found {
        println!("No sessions found.");
    }

    Ok(())
}

/// Close a session by setting its `closed` flag to true.
pub fn sessions_close(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    let path = session_dir().join(format!("{key}.json"));
    let mut record = match SessionRecord::load(&path).map_err(AcpCliError::Io)? {
        Some(r) => r,
        None => {
            return Err(AcpCliError::NoSession {
                agent: agent.to_string(),
                cwd: cwd.to_string(),
            });
        }
    };

    if record.closed {
        println!("Session is already closed.");
        return Ok(());
    }

    record.closed = true;
    record.save(&path).map_err(AcpCliError::Io)?;

    let short_id = &record.id[..12.min(record.id.len())];
    println!("Session {short_id} closed.");
    Ok(())
}

/// Show detailed information about a session.
pub fn sessions_show(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    let path = session_dir().join(format!("{key}.json"));
    let record = match SessionRecord::load(&path).map_err(AcpCliError::Io)? {
        Some(r) => r,
        None => {
            return Err(AcpCliError::NoSession {
                agent: agent.to_string(),
                cwd: cwd.to_string(),
            });
        }
    };

    let status = if record.closed { "closed" } else { "open" };
    let created = format_timestamp(record.created_at);

    println!("ID:         {}", record.id);
    println!("Agent:      {}", record.agent);
    println!("CWD:        {}", record.cwd.display());
    if let Some(ref n) = record.name {
        println!("Name:       {n}");
    }
    println!("Created at: {created}");
    println!("Status:     {status}");
    if let Some(ref acp_id) = record.acp_session_id {
        println!("ACP Session: {acp_id}");
    }

    Ok(())
}

/// Show conversation history for a session.
pub fn sessions_history(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    // Verify the session exists
    let sess_path = session_dir().join(format!("{key}.json"));
    if SessionRecord::load(&sess_path)
        .map_err(AcpCliError::Io)?
        .is_none()
    {
        return Err(AcpCliError::NoSession {
            agent: agent.to_string(),
            cwd: cwd.to_string(),
        });
    }

    let entries = load_history(&key).map_err(AcpCliError::Io)?;

    if entries.is_empty() {
        println!("No conversation history.");
        return Ok(());
    }

    for entry in &entries {
        let ts = format_timestamp(entry.timestamp);
        println!("[{ts}] {}:", entry.role);
        println!("{}", entry.content);
        println!();
    }

    Ok(())
}

/// Cancel a running prompt by sending SIGTERM to the active process.
pub fn cancel_prompt(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    match pid::read_pid(&key) {
        Some(active_pid) => {
            // Send SIGTERM to the running process
            // SAFETY: sending SIGTERM to a known-alive PID is standard POSIX behavior.
            let ret = unsafe { libc::kill(active_pid as libc::pid_t, libc::SIGTERM) };
            if ret == 0 {
                println!("Sent SIGTERM to process {active_pid}.");
            } else {
                eprintln!("Failed to send signal to process {active_pid}.");
            }
            Ok(())
        }
        None => {
            println!("No active prompt.");
            Ok(())
        }
    }
}

/// Show the status of the current session, including whether a prompt is running.
pub fn session_status(agent: &str, cwd: &str, name: Option<&str>) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    let sess_path = session_dir().join(format!("{key}.json"));
    let record = match SessionRecord::load(&sess_path).map_err(AcpCliError::Io)? {
        Some(r) => r,
        None => {
            return Err(AcpCliError::NoSession {
                agent: agent.to_string(),
                cwd: cwd.to_string(),
            });
        }
    };

    let active_pid = pid::read_pid(&key);
    let status = if record.closed {
        "closed"
    } else if active_pid.is_some() {
        "running"
    } else {
        "idle"
    };

    println!("Agent:   {}", record.agent);
    println!("Session: {}", record.id);
    println!("Status:  {status}");
    if let Some(p) = active_pid {
        println!("PID:     {p}");
    }

    Ok(())
}

/// Set the session mode by sending a request to the queue owner via IPC.
///
/// Requires an active session with a running queue owner process.
pub async fn set_mode(agent: &str, cwd: &str, name: Option<&str>, mode: &str) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let key = session_key(agent, &dir_str, session_name);

    let mut client = crate::queue::client::QueueClient::connect(&key)
        .await
        .map_err(|_| AcpCliError::Connection("No active session. Start a prompt first.".into()))?;

    client.set_mode(mode).await
}

/// Set a session config option by sending a request to the queue owner via IPC.
///
/// Requires an active session with a running queue owner process.
pub async fn set_config(
    agent: &str,
    cwd: &str,
    name: Option<&str>,
    key: &str,
    value: &str,
) -> Result<()> {
    let cwd_path = Path::new(cwd);
    let resolved_dir = find_git_root(cwd_path).unwrap_or_else(|| cwd_path.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let session_name = name.unwrap_or("");
    let session_key = session_key(agent, &dir_str, session_name);

    let mut client = crate::queue::client::QueueClient::connect(&session_key)
        .await
        .map_err(|_| AcpCliError::Connection("No active session. Start a prompt first.".into()))?;

    client.set_config(key, value).await
}

/// Format a Unix timestamp as `YYYY-MM-DD HH:MM:SS`.
fn format_timestamp(ts: u64) -> String {
    let secs = ts;
    // Simple UTC formatting without external crate
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Civil date from days since 1970-01-01 (algorithm from Howard Hinnant)
    let z = days_since_epoch as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02}")
}
