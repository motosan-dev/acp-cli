use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AcpCliError, Result};
use crate::session::persistence::SessionRecord;
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
