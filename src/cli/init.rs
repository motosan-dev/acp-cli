use std::io::{self, BufRead, Write};

use crate::config::AcpCliConfig;

/// Run the interactive init flow.
pub fn run_init() -> crate::error::Result<()> {
    println!("acp-cli init\n");

    // 1. Check Claude Code installation
    print!("🔍 Checking Claude Code installation... ");
    io::stdout().flush().ok();
    let has_claude = which_command("claude");
    if has_claude {
        println!("✅ found");
    } else {
        println!("⚠️  not found (optional — install from https://claude.ai/code)");
    }

    // 2. Check npx (needed for claude-agent-acp)
    print!("🔍 Checking npx... ");
    io::stdout().flush().ok();
    if which_command("npx") {
        println!("✅ found");
    } else {
        println!("❌ not found");
        println!("   Hint: Install Node.js — https://nodejs.org/");
    }

    // 3. Detect existing auth token
    println!("\n🔍 Checking auth token...");

    let mut detected_token: Option<String> = None;
    let mut token_source = "";

    // env var
    if let Ok(t) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
        if !t.is_empty() {
            println!(
                "   ANTHROPIC_AUTH_TOKEN env: ✅ found ({}...)",
                mask_token(&t)
            );
            detected_token = Some(t);
            token_source = "env var";
        }
    }

    // existing config
    if detected_token.is_none() {
        let existing = AcpCliConfig::load();
        if let Some(ref t) = existing.auth_token {
            if !t.is_empty() {
                println!(
                    "   ~/.acp-cli/config.json:   ✅ found ({}...)",
                    mask_token(t)
                );
                detected_token = Some(t.clone());
                token_source = "config";
            }
        }
    }

    // ~/.claude.json
    if detected_token.is_none() {
        if let Some(t) = read_claude_json_token() {
            println!(
                "   ~/.claude.json:           ✅ found ({}...)",
                mask_token(&t)
            );
            detected_token = Some(t);
            token_source = "~/.claude.json";
        }
    }

    // macOS Keychain
    #[cfg(target_os = "macos")]
    if detected_token.is_none() {
        if let Some(t) = read_keychain_token() {
            println!(
                "   macOS Keychain:           ✅ found ({}...)",
                mask_token(&t)
            );
            detected_token = Some(t);
            token_source = "Keychain";
        }
    }

    if detected_token.is_none() {
        println!("   No token detected.");
    }

    // 4. Ask user what to do
    let final_token = if let Some(ref token) = detected_token {
        println!("\nDetected token from {token_source}.");
        print!("Save to config? [Y/n] ");
        io::stdout().flush().ok();
        let answer = read_line_trim();
        if answer.is_empty() || answer.to_lowercase().starts_with('y') {
            Some(token.clone())
        } else {
            print!("\nEnter auth token (or press Enter to skip): ");
            io::stdout().flush().ok();
            let input = read_line_trim();
            if input.is_empty() { None } else { Some(input) }
        }
    } else {
        print!("\nEnter your Anthropic auth token (or press Enter to skip): ");
        io::stdout().flush().ok();
        let input = read_line_trim();
        if input.is_empty() { None } else { Some(input) }
    };

    // 5. Write config
    let config_dir = dirs::home_dir()
        .ok_or_else(|| crate::error::AcpCliError::Usage("cannot find home directory".into()))?
        .join(".acp-cli");

    std::fs::create_dir_all(&config_dir).map_err(|e| {
        crate::error::AcpCliError::Usage(format!("failed to create {}: {e}", config_dir.display()))
    })?;

    let config_path = config_dir.join("config.json");

    // Load existing config to preserve other fields
    let mut config = AcpCliConfig::load();

    if let Some(token) = final_token {
        config.auth_token = Some(token);
    }

    // Set default agent if not already set
    if config.default_agent.is_none() {
        config.default_agent = Some("claude".to_string());
    }

    let json = serde_json::to_string_pretty(&config).map_err(|e| {
        crate::error::AcpCliError::Usage(format!("failed to serialize config: {e}"))
    })?;

    std::fs::write(&config_path, &json).map_err(|e| {
        crate::error::AcpCliError::Usage(format!("failed to write {}: {e}", config_path.display()))
    })?;

    println!("\n✅ Config written to {}", config_path.display());

    if config.auth_token.is_some() {
        println!("✅ Auth token saved");
    } else {
        println!("⚠️  No auth token configured — set ANTHROPIC_AUTH_TOKEN or re-run init");
    }

    println!(
        "✅ Default agent: {}",
        config.default_agent.as_deref().unwrap_or("claude")
    );

    Ok(())
}

fn read_line_trim() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    line.trim().to_string()
}

fn mask_token(token: &str) -> String {
    if token.len() <= 12 {
        return "***".to_string();
    }
    format!("{}...{}", &token[..8], &token[token.len() - 4..])
}

fn which_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_claude_json_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".claude.json");
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json.pointer("/oauthAccount/accessToken")
        .or_else(|| json.get("accessToken"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(target_os = "macos")]
fn read_keychain_token() -> Option<String> {
    for service in &["Claude Code", "claude.ai", "anthropic.claude"] {
        let output = std::process::Command::new("security")
            .args(["find-generic-password", "-s", service, "-w"])
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if output.status.success() {
            let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}
