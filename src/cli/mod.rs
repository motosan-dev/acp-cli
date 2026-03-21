pub mod prompt;
pub mod prompt_source;
pub mod session;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "acp-cli",
    version,
    about = "Headless CLI client for the Agent Client Protocol"
)]
pub struct Cli {
    /// Agent name (e.g. "claude", "codex") or raw command
    pub agent: Option<String>,

    /// Prompt text (implicit prompt mode)
    pub prompt: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Session name or ID to resume
    #[arg(short = 's', long)]
    pub session: Option<String>,

    /// Automatically approve all tool calls
    #[arg(long)]
    pub approve_all: bool,

    /// Automatically approve read-only tool calls
    #[arg(long)]
    pub approve_reads: bool,

    /// Deny all tool calls
    #[arg(long)]
    pub deny_all: bool,

    /// Working directory for the agent
    #[arg(long)]
    pub cwd: Option<String>,

    /// Output format (text, json, quiet)
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Timeout in seconds
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Override the agent command
    #[arg(long = "agent-override")]
    pub agent_override: Option<String>,

    /// Read prompt from a file (use "-" for stdin)
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Enable verbose output
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Execute a prompt non-interactively
    Exec {
        /// Prompt text
        prompt: Vec<String>,
    },
    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum SessionAction {
    /// Create a new session
    New {
        /// Optional session name
        #[arg(long)]
        name: Option<String>,
    },
    /// List existing sessions
    List,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
}
