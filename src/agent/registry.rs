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

    m.insert(
        "claude".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/claude-code-acp@latest".into()],
            description: "Claude Code (Anthropic)".into(),
        },
    );

    m.insert(
        "codex".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@openai/codex-acp@latest".into()],
            description: "Codex (OpenAI)".into(),
        },
    );

    m.insert(
        "gemini".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/gemini-acp@latest".into()],
            description: "Gemini (Google)".into(),
        },
    );

    m.insert(
        "pi".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/pi-acp@latest".into()],
            description: "Pi (Inflection)".into(),
        },
    );

    m.insert(
        "openclaw".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/openclaw-acp@latest".into()],
            description: "OpenClaw".into(),
        },
    );

    m.insert(
        "goose".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/goose-acp@latest".into()],
            description: "Goose (Block)".into(),
        },
    );

    m.insert(
        "kiro".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/kiro-acp@latest".into()],
            description: "Kiro (AWS)".into(),
        },
    );

    m.insert(
        "opencode".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/opencode-acp@latest".into()],
            description: "OpenCode".into(),
        },
    );

    m.insert(
        "copilot".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/copilot-acp@latest".into()],
            description: "Copilot (GitHub)".into(),
        },
    );

    m.insert(
        "cursor".into(),
        AgentEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@anthropic-ai/cursor-acp@latest".into()],
            description: "Cursor".into(),
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
