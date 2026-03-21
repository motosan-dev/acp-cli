use acp_cli::session::persistence::SessionRecord;
use acp_cli::session::scoping::session_key;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn session_key_deterministic() {
    let k1 = session_key("claude", "/home/user/project", "default");
    let k2 = session_key("claude", "/home/user/project", "default");
    assert_eq!(k1, k2);
    // SHA-256 hex is always 64 characters
    assert_eq!(k1.len(), 64);
}

#[test]
fn session_key_differs_by_name() {
    let k1 = session_key("claude", "/home/user/project", "alpha");
    let k2 = session_key("claude", "/home/user/project", "beta");
    assert_ne!(k1, k2);
}

#[test]
fn session_key_no_collision() {
    // The null separator prevents ("a", "b\0c", "") from colliding with ("a\0b", "c", "").
    // We test a simpler but analogous case: ("ab", "cd", "") vs ("a", "bcd", "").
    let k1 = session_key("ab", "cd", "");
    let k2 = session_key("a", "bcd", "");
    assert_ne!(k1, k2);

    // Also verify shifted boundaries with name field
    let k3 = session_key("agent", "dir", "name");
    let k4 = session_key("agent", "dirname", "");
    assert_ne!(k3, k4);
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("session.json");

    let record = SessionRecord {
        id: "sess-001".to_string(),
        agent: "claude".to_string(),
        cwd: PathBuf::from("/home/user/project"),
        name: Some("my-session".to_string()),
        created_at: 1700000000,
        closed: false,
        acp_session_id: None,
    };

    record.save(&path).unwrap();
    let loaded = SessionRecord::load(&path)
        .unwrap()
        .expect("should load saved session");

    assert_eq!(loaded.id, record.id);
    assert_eq!(loaded.agent, record.agent);
    assert_eq!(loaded.cwd, record.cwd);
    assert_eq!(loaded.name, record.name);
    assert_eq!(loaded.created_at, record.created_at);
    assert_eq!(loaded.closed, record.closed);
}

#[test]
fn load_missing_returns_none() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("does-not-exist.json");

    let result = SessionRecord::load(&path).unwrap();
    assert!(result.is_none());
}
