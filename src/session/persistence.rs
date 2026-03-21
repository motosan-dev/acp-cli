use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub agent: String,
    pub cwd: PathBuf,
    pub name: Option<String>,
    pub created_at: u64,
    pub closed: bool,
    /// The latest ACP session ID from the bridge (updated on each reconnect).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_session_id: Option<String>,
}

impl SessionRecord {
    /// Write this session record as JSON to the given file path.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Update the ACP session ID (for when the agent is re-spawned) and persist.
    pub fn update_acp_session_id(&mut self, new_id: String, path: &Path) -> io::Result<()> {
        self.acp_session_id = Some(new_id);
        self.save(path)
    }

    /// Load a session record from a JSON file. Returns `None` if the file does not exist.
    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let record: Self = serde_json::from_str(&contents)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Some(record))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}
