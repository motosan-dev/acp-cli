use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A built-in or discovered agent entry in the registry.
#[derive(Debug, Clone)]
pub struct AgentEntry {
    pub command: String,
    pub args: Vec<String>,
    pub description: String,
}

/// User-provided override for an agent's command and args (from config file).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentOverride {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Returns the default registry of built-in agents.
pub fn default_registry() -> HashMap<String, AgentEntry> {
    let mut m = HashMap::new();

    // --- Agents with npm ACP adapters ---
    m.insert(
        "claude".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@zed-industries/claude-agent-acp".into()],
            description: "Claude Code via ACP bridge".into(),
        },
    );
    m.insert(
        "codex".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@zed-industries/codex-acp".into()],
            description: "OpenAI Codex CLI".into(),
        },
    );
    m.insert(
        "pi".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "pi-acp".into()],
            description: "Pi Coding Agent".into(),
        },
    );
    m.insert(
        "kilocode".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@kilocode/cli".into(), "acp".into()],
            description: "Kilocode".into(),
        },
    );
    m.insert(
        "opencode".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "opencode-ai".into(), "acp".into()],
            description: "OpenCode".into(),
        },
    );

    // --- Agents with native ACP support ---
    m.insert(
        "gemini".into(),
        AgentEntry {
            command: "gemini".into(),
            args: vec!["--acp".into()],
            description: "Google Gemini CLI".into(),
        },
    );
    m.insert(
        "openclaw".into(),
        AgentEntry {
            command: "openclaw".into(),
            args: vec!["acp".into()],
            description: "OpenClaw".into(),
        },
    );
    m.insert(
        "cursor".into(),
        AgentEntry {
            command: "cursor-agent".into(),
            args: vec!["acp".into()],
            description: "Cursor".into(),
        },
    );
    m.insert(
        "copilot".into(),
        AgentEntry {
            command: "copilot".into(),
            args: vec!["--acp".into(), "--stdio".into()],
            description: "GitHub Copilot".into(),
        },
    );
    m.insert(
        "kiro".into(),
        AgentEntry {
            command: "kiro-cli".into(),
            args: vec!["acp".into()],
            description: "Kiro CLI (AWS)".into(),
        },
    );
    m.insert(
        "kimi".into(),
        AgentEntry {
            command: "kimi".into(),
            args: vec!["acp".into()],
            description: "Kimi CLI".into(),
        },
    );
    m.insert(
        "qwen".into(),
        AgentEntry {
            command: "qwen".into(),
            args: vec!["--acp".into()],
            description: "Qwen Code".into(),
        },
    );
    m.insert(
        "droid".into(),
        AgentEntry {
            command: "droid".into(),
            args: vec!["exec".into(), "--output-format".into(), "acp".into()],
            description: "Factory Droid".into(),
        },
    );
    m.insert(
        "goose".into(),
        AgentEntry {
            command: "goose".into(),
            args: vec!["acp".into()],
            description: "Goose (Block)".into(),
        },
    );

    m
}

/// Resolves an agent name to a `(command, args)` pair.
///
/// Resolution order:
/// 1. Check `overrides` (user config) first
/// 2. Check the built-in `registry`
/// 3. Treat the name as a raw command with no args
pub fn resolve_agent(
    name: &str,
    registry: &HashMap<String, AgentEntry>,
    overrides: &HashMap<String, AgentOverride>,
) -> (String, Vec<String>) {
    if let Some(ov) = overrides.get(name) {
        return (ov.command.clone(), ov.args.clone());
    }

    if let Some(entry) = registry.get(name) {
        return (entry.command.clone(), entry.args.clone());
    }

    // Fallback: treat the name itself as a raw command
    (name.to_string(), Vec::new())
}
