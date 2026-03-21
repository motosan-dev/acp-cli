use std::collections::HashMap;

use acp_cli::agent::registry::{AgentOverride, default_registry, resolve_agent};

#[test]
fn resolves_claude() {
    let registry = default_registry();
    let overrides = HashMap::new();

    let (cmd, args) = resolve_agent("claude", &registry, &overrides);

    assert_eq!(cmd, "npx");
    assert!(args.contains(&"-y".to_string()));
    assert!(args.iter().any(|a| a.contains("claude")));
}

#[test]
fn resolves_unknown_as_raw_command() {
    let registry = default_registry();
    let overrides = HashMap::new();

    let (cmd, args) = resolve_agent("my-custom-agent", &registry, &overrides);

    assert_eq!(cmd, "my-custom-agent");
    assert!(args.is_empty());
}

#[test]
fn config_override_wins() {
    let registry = default_registry();
    let mut overrides = HashMap::new();
    overrides.insert(
        "claude".to_string(),
        AgentOverride {
            command: "/usr/local/bin/my-claude".to_string(),
            args: vec!["--custom".to_string()],
        },
    );

    let (cmd, args) = resolve_agent("claude", &registry, &overrides);

    assert_eq!(cmd, "/usr/local/bin/my-claude");
    assert_eq!(args, vec!["--custom"]);
}
