use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};

use super::scoping::session_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

/// Return the path to the conversation log file for a given session key.
fn log_path(session_key: &str) -> std::path::PathBuf {
    session_dir().join(format!("{session_key}.log.jsonl"))
}

/// Append an entry to the conversation log file.
pub fn append_entry(session_key: &str, entry: &ConversationEntry) -> io::Result<()> {
    let path = log_path(session_key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    let json =
        serde_json::to_string(entry).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(file, "{json}")?;
    Ok(())
}

/// Load all entries from the conversation log file.
pub fn load_history(session_key: &str) -> io::Result<Vec<ConversationEntry>> {
    let path = log_path(session_key);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let reader = io::BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: ConversationEntry = serde_json::from_str(&line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        entries.push(entry);
    }
    Ok(entries)
}
