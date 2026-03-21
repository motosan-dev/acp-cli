use acp_cli::agent::registry::AgentOverride;
use acp_cli::client::permissions::PermissionMode;
use acp_cli::config::AcpCliConfig;
use std::collections::HashMap;
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

// --- merge tests ---

#[test]
fn merge_project_overrides_global() {
    let global = AcpCliConfig {
        default_agent: Some("claude".into()),
        timeout: Some(60),
        format: Some("text".into()),
        ..Default::default()
    };
    let project = AcpCliConfig {
        default_agent: Some("gpt".into()),
        timeout: None,
        format: Some("json".into()),
        ..Default::default()
    };

    let merged = global.merge(project);
    assert_eq!(merged.default_agent.as_deref(), Some("gpt"));
    assert_eq!(merged.timeout, Some(60)); // global preserved when project is None
    assert_eq!(merged.format.as_deref(), Some("json")); // project wins
}

#[test]
fn merge_both_default_yields_default() {
    let merged = AcpCliConfig::default().merge(AcpCliConfig::default());
    assert!(merged.default_agent.is_none());
    assert!(merged.timeout.is_none());
    assert!(merged.format.is_none());
    assert!(merged.default_permissions.is_none());
    assert!(merged.agents.is_none());
}

#[test]
fn merge_agents_maps_are_combined() {
    let mut global_agents = HashMap::new();
    global_agents.insert(
        "a".into(),
        AgentOverride {
            command: "/usr/bin/a".into(),
            args: vec![],
        },
    );
    global_agents.insert(
        "b".into(),
        AgentOverride {
            command: "/usr/bin/b-global".into(),
            args: vec![],
        },
    );

    let mut project_agents = HashMap::new();
    project_agents.insert(
        "b".into(),
        AgentOverride {
            command: "/usr/bin/b-project".into(),
            args: vec!["--fast".into()],
        },
    );
    project_agents.insert(
        "c".into(),
        AgentOverride {
            command: "/usr/bin/c".into(),
            args: vec![],
        },
    );

    let global = AcpCliConfig {
        agents: Some(global_agents),
        ..Default::default()
    };
    let project = AcpCliConfig {
        agents: Some(project_agents),
        ..Default::default()
    };

    let merged = global.merge(project);
    let agents = merged.agents.unwrap();
    assert_eq!(agents.len(), 3); // a, b (project wins), c
    assert_eq!(agents["a"].command, "/usr/bin/a");
    assert_eq!(agents["b"].command, "/usr/bin/b-project"); // project overrides
    assert_eq!(agents["c"].command, "/usr/bin/c");
}

#[test]
fn merge_permissions_project_wins() {
    let global = AcpCliConfig {
        default_permissions: Some(PermissionMode::DenyAll),
        ..Default::default()
    };
    let project = AcpCliConfig {
        default_permissions: Some(PermissionMode::ApproveAll),
        ..Default::default()
    };

    let merged = global.merge(project);
    assert!(matches!(
        merged.default_permissions,
        Some(PermissionMode::ApproveAll)
    ));
}

#[test]
fn load_project_from_git_root() {
    let dir = tempfile::tempdir().unwrap();
    // Create a fake git root
    std::fs::create_dir(dir.path().join(".git")).unwrap();

    // Write .acp-cli.json in the root
    let config_path = dir.path().join(".acp-cli.json");
    std::fs::write(
        &config_path,
        r#"{"default_agent": "local-agent", "timeout": 30}"#,
    )
    .unwrap();

    // Create a subdirectory to load from
    let sub = dir.path().join("src").join("deep");
    std::fs::create_dir_all(&sub).unwrap();

    let cfg = AcpCliConfig::load_project(&sub);
    assert_eq!(cfg.default_agent.as_deref(), Some("local-agent"));
    assert_eq!(cfg.timeout, Some(30));
}

#[test]
fn load_project_no_git_root_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    // No .git directory
    let cfg = AcpCliConfig::load_project(dir.path());
    assert!(cfg.default_agent.is_none());
}
