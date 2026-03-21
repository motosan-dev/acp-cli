use serde::{Deserialize, Serialize};

use crate::session::scoping::session_dir;

/// Lease file stored at `~/.acp-cli/sessions/<key>.lease`.
///
/// The queue owner writes this file on startup and updates the heartbeat
/// periodically. Other processes read it to determine whether a live owner
/// already exists for a given session key.
#[derive(Debug, Serialize, Deserialize)]
pub struct LeaseFile {
    /// PID of the queue owner process.
    pub pid: u32,
    /// Unix timestamp (seconds) when the owner started.
    pub start_time: u64,
    /// Unix timestamp (seconds) of the last heartbeat update.
    pub last_heartbeat: u64,
}

impl LeaseFile {
    /// Write a new lease file for `session_key` with the current PID and timestamp.
    pub fn write(session_key: &str) -> std::io::Result<()> {
        let path = lease_path(session_key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let now = now_secs();
        let lease = LeaseFile {
            pid: std::process::id(),
            start_time: now,
            last_heartbeat: now,
        };
        let json = serde_json::to_string_pretty(&lease)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    /// Read the lease file for `session_key`. Returns `None` if the file is
    /// missing or cannot be parsed.
    pub fn read(session_key: &str) -> Option<LeaseFile> {
        let path = lease_path(session_key);
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Update the `last_heartbeat` timestamp in an existing lease file.
    pub fn update_heartbeat(session_key: &str) -> std::io::Result<()> {
        let path = lease_path(session_key);
        let contents = std::fs::read_to_string(&path)?;
        let mut lease: LeaseFile = serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        lease.last_heartbeat = now_secs();
        let json = serde_json::to_string_pretty(&lease)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    /// Remove the lease file for `session_key` (best-effort).
    pub fn remove(session_key: &str) {
        let _ = std::fs::remove_file(lease_path(session_key));
    }

    /// Check whether the lease is still valid:
    /// - The heartbeat is within `ttl_secs` of the current time.
    /// - The process identified by `pid` is still alive.
    pub fn is_valid(&self, ttl_secs: u64) -> bool {
        let now = now_secs();
        if now.saturating_sub(self.last_heartbeat) > ttl_secs {
            return false;
        }
        is_process_alive(self.pid)
    }
}

/// Return the lease file path for a session key.
fn lease_path(session_key: &str) -> std::path::PathBuf {
    session_dir().join(format!("{session_key}.lease"))
}

/// Current time as seconds since the Unix epoch.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check whether a process with the given PID is alive (POSIX `kill(pid, 0)`).
fn is_process_alive(pid: u32) -> bool {
    let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if ret == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_alive() {
        assert!(is_process_alive(std::process::id()));
    }

    #[test]
    fn expired_lease_is_invalid() {
        let lease = LeaseFile {
            pid: std::process::id(),
            start_time: 1000,
            last_heartbeat: 1000,
        };
        // TTL of 60s, but heartbeat is from epoch time 1000 — long expired.
        assert!(!lease.is_valid(60));
    }

    #[test]
    fn fresh_lease_with_live_pid_is_valid() {
        let now = now_secs();
        let lease = LeaseFile {
            pid: std::process::id(),
            start_time: now,
            last_heartbeat: now,
        };
        assert!(lease.is_valid(60));
    }

    #[test]
    fn lease_with_dead_pid_is_invalid() {
        let now = now_secs();
        let lease = LeaseFile {
            pid: 4_000_000, // almost certainly unused
            start_time: now,
            last_heartbeat: now,
        };
        assert!(!lease.is_valid(60));
    }
}
