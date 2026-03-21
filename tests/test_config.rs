use acp_cli::config::AcpCliConfig;
use std::io::Write;

#[test]
fn default_config() {
    let cfg = AcpCliConfig::default();
    assert!(cfg.default_agent.is_none());
    assert!(cfg.default_permissions.is_none());
    assert!(cfg.timeout.is_none());
    assert!(cfg.format.is_none());
    assert!(cfg.agents.is_none());
}

#[test]
fn deserialize_config() {
    let json = r#"{
        "default_agent": "claude",
        "default_permissions": "approve_all",
        "timeout": 120,
        "format": "json",
        "agents": {
            "my-agent": {
                "command": "/usr/local/bin/my-agent",
                "args": ["--verbose"]
            }
        }
    }"#;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(json.as_bytes()).unwrap();

    let cfg = AcpCliConfig::load_from(&path);
    assert_eq!(cfg.default_agent.as_deref(), Some("claude"));
    assert!(cfg.default_permissions.is_some());
    assert_eq!(cfg.timeout, Some(120));
    assert_eq!(cfg.format.as_deref(), Some("json"));

    let agents = cfg.agents.unwrap();
    assert!(agents.contains_key("my-agent"));
    let ov = &agents["my-agent"];
    assert_eq!(ov.command, "/usr/local/bin/my-agent");
    assert_eq!(ov.args, vec!["--verbose"]);
}

#[test]
fn load_missing_file_returns_default() {
    let cfg = AcpCliConfig::load_from("/tmp/nonexistent-acp-cli-config-12345.json");
    assert!(cfg.default_agent.is_none());
    assert!(cfg.timeout.is_none());
}
