use std::path::PathBuf;

use super::scoping::session_dir;

/// Return the path for a session's PID file: `~/.acp-cli/sessions/<key>.pid`.
fn pid_path(session_key: &str) -> PathBuf {
    session_dir().join(format!("{session_key}.pid"))
}

/// Write the current process PID to the pid file for a session key.
pub fn write_pid(session_key: &str) -> std::io::Result<()> {
    let path = pid_path(session_key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, std::process::id().to_string())
}

/// Remove the pid file for a session key.
pub fn remove_pid(session_key: &str) -> std::io::Result<()> {
    let path = pid_path(session_key);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Read the PID from the pid file and check if the process is alive.
///
/// Returns `Some(pid)` if the file exists and the process is running,
/// `None` if the file is missing, unreadable, or the process is dead.
pub fn read_pid(session_key: &str) -> Option<u32> {
    let path = pid_path(session_key);
    let contents = std::fs::read_to_string(&path).ok()?;
    let pid: u32 = contents.trim().parse().ok()?;
    if is_process_alive(pid) {
        Some(pid)
    } else {
        // Stale PID file — clean it up
        let _ = std::fs::remove_file(&path);
        None
    }
}

/// Check if a process with the given PID is alive using `kill(pid, 0)`.
///
/// Signal 0 checks for process existence without actually sending a signal.
fn is_process_alive(pid: u32) -> bool {
    // kill with signal 0 is a standard POSIX existence check.
    // Returns 0 if the process exists and we have permission to signal it.
    // Returns -1 with ESRCH if the process does not exist,
    // or -1 with EPERM if it exists but we lack permission (still alive).
    let ret = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if ret == 0 {
        return true;
    }
    // Use std::io::Error to portably retrieve errno
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
    fn bogus_pid_is_not_alive() {
        // PID 4_000_000 is almost certainly unused
        assert!(!is_process_alive(4_000_000));
    }
}
