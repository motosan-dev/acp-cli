use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::persistence::SessionRecord;

/// Compute a deterministic session key from agent, directory, and optional name.
///
/// Uses SHA-256 of `"agent\0dir\0name"` (hex encoded). The null byte separator
/// prevents collisions between e.g. `("a","b\0c","")` and `("a\0b","c","")`.
pub fn session_key(agent: &str, dir: &str, name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(agent.as_bytes());
    hasher.update(b"\0");
    hasher.update(dir.as_bytes());
    hasher.update(b"\0");
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

/// Walk up from `from` looking for a `.git` directory, returning the git root.
pub fn find_git_root(from: &Path) -> Option<PathBuf> {
    let mut current = from.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Return the default session storage directory: `~/.acp-cli/sessions/`.
pub fn session_dir() -> PathBuf {
    let home = dirs::home_dir().expect("could not determine home directory");
    home.join(".acp-cli").join("sessions")
}

/// Find an existing session record for the given agent, working directory, and name.
///
/// Resolves the directory to a git root (if possible), computes the session key,
/// and attempts to load the corresponding session file.
pub fn find_session(agent: &str, cwd: &Path, name: &str) -> Option<SessionRecord> {
    let resolved_dir = find_git_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let dir_str = resolved_dir.to_string_lossy();
    let key = session_key(agent, &dir_str, name);
    let path = session_dir().join(format!("{key}.json"));
    SessionRecord::load(&path).ok().flatten()
}
