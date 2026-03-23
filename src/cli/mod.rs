pub mod init;
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

    /// Fire-and-forget: queue the prompt and return immediately without waiting
    /// for the result. Requires an active session (queue owner) to be running.
    #[arg(long)]
    pub no_wait: bool,
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
    /// Interactive setup — detect auth token, write config
    Init,
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Cancel a running prompt by sending SIGTERM
    Cancel,
    /// Show status of the current session (running/idle/closed)
    Status,
    /// Set the session mode (e.g. "code", "plan", "chat")
    SetMode {
        /// Mode identifier to set
        mode: String,
    },
    /// Set a session configuration option
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
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
    /// Close a session
    Close {
        /// Session name
        #[arg(short = 's', long)]
        name: Option<String>,
    },
    /// Show session details
    Show {
        /// Session name
        #[arg(short = 's', long)]
        name: Option<String>,
    },
    /// Show conversation history
    History {
        /// Session name
        #[arg(short = 's', long)]
        name: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
}
