//! Integration tests for the acp-cli binary.
//! Tests CLI commands via subprocess execution.

use std::process::Command;

fn acp_cli() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_acp-cli"));
    // Prevent loading user's real config
    cmd.env("HOME", "/tmp/acp-cli-test-nonexistent");
    cmd
}

// --- help / version ---

#[test]
fn help_flag_shows_usage() {
    let output = acp_cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Headless CLI client"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("exec"));
    assert!(stdout.contains("sessions"));
    assert!(stdout.contains("config"));
}

#[test]
fn version_flag_shows_version() {
    let output = acp_cli().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("acp-cli"));
}

// --- config show ---

#[test]
fn config_show_outputs_json() {
    let output = acp_cli().args(["config", "show"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should be valid JSON (even if default/empty)
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_object());
}

// --- config show with custom config ---

#[test]
fn config_show_reads_auth_token() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".acp-cli");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("config.json"),
        r#"{"default_agent":"codex","auth_token":"sk-test-123"}"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_acp-cli"))
        .env("HOME", dir.path())
        .args(["config", "show"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["default_agent"], "codex");
    assert_eq!(parsed["auth_token"], "sk-test-123");
}

// --- init (non-interactive, piped empty stdin) ---

#[test]
fn init_creates_config_file() {
    let dir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_acp-cli"))
        .env("HOME", dir.path())
        .env_remove("ANTHROPIC_AUTH_TOKEN")
        .arg("init")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Config written to"));

    // Config file should exist
    let config_path = dir.path().join(".acp-cli").join("config.json");
    assert!(config_path.exists());

    // Should have default_agent = claude
    let content = std::fs::read_to_string(&config_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["default_agent"], "claude");
}

// --- init with token in env ---

#[test]
fn init_detects_env_token() {
    let dir = tempfile::tempdir().unwrap();

    // Pipe "y\n" to stdin to accept saving
    let mut child = Command::new(env!("CARGO_BIN_EXE_acp-cli"))
        .env("HOME", dir.path())
        .env("ANTHROPIC_AUTH_TOKEN", "sk-ant-test-token-xyz")
        .arg("init")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Write "y" to stdin
    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(b"y\n").ok();
    }

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ANTHROPIC_AUTH_TOKEN env"));
    assert!(stdout.contains("Auth token saved"));

    // Verify token written to config
    let config_path = dir.path().join(".acp-cli").join("config.json");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["auth_token"], "sk-ant-test-token-xyz");
}

// --- sessions list (empty) ---

#[test]
fn sessions_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    // Create sessions dir so it doesn't error
    std::fs::create_dir_all(dir.path().join(".acp-cli").join("sessions")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_acp-cli"))
        .env("HOME", dir.path())
        .args(["claude", "sessions", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should show header or "No sessions" message
    assert!(stdout.contains("ID") || stdout.contains("No sessions") || stdout.is_empty());
}

// --- no args shows help ---

#[test]
fn no_args_shows_help() {
    let output = acp_cli().output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage:") || stdout.contains("acp-cli"));
}

// --- --prompt-retries flag ---

#[test]
fn help_shows_prompt_retries_flag() {
    let output = acp_cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("prompt-retries"));
}

#[test]
fn prompt_retries_default_is_zero() {
    use acp_cli::cli::Cli;
    use clap::Parser;
    let cli = Cli::parse_from(["acp-cli", "config", "show"]);
    assert_eq!(cli.prompt_retries, 0);
}

#[test]
fn prompt_retries_flag_is_accepted() {
    // Verify the flag is parsed without error (no agent/prompt means help is shown).
    let output = acp_cli()
        .args(["--prompt-retries", "3", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("prompt-retries"));
}

// --- --suppress-reads flag ---

#[test]
fn help_shows_suppress_reads_flag() {
    let output = acp_cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("suppress-reads"));
}

#[test]
fn suppress_reads_default_is_false() {
    use acp_cli::cli::Cli;
    use clap::Parser;
    let cli = Cli::parse_from(["acp-cli", "config", "show"]);
    assert!(!cli.suppress_reads);
}

#[test]
fn suppress_reads_flag_is_accepted() {
    let output = acp_cli()
        .args(["--suppress-reads", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("suppress-reads"));
}

#[test]
fn suppress_reads_parses_to_true() {
    use acp_cli::cli::Cli;
    use clap::Parser;
    let cli = Cli::parse_from(["acp-cli", "--suppress-reads", "config", "show"]);
    assert!(cli.suppress_reads);
}
