# acp-cli

Headless CLI client for the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/). Talk to coding agents (Claude, Codex, Gemini, etc.) over structured JSON-RPC instead of terminal scraping.

Rust port of [ACPX](https://github.com/openclaw/acpx).

## Install

```bash
cargo install acp-cli
```

## Quick Start

```bash
# 1. Setup (detect auth, write config)
acp-cli init

# 2. Talk to Claude
acp-cli claude "fix the auth bug" --approve-all
```

## Usage

```bash
# Prompt (persistent session)
acp-cli claude "fix the auth bug"

# One-shot (no session persistence)
acp-cli claude exec "what does this function do?"

# Read prompt from file
acp-cli claude -f prompt.md --approve-all

# Pipe from stdin
echo "fix the bug" | acp-cli claude --approve-all

# Different agents
acp-cli codex "refactor this module"
acp-cli gemini "explain this error"

# Named sessions for parallel work
acp-cli claude -s backend "fix the API"
acp-cli claude -s frontend "update the UI"

# Output formats
acp-cli claude "list TODOs" --format json     # NDJSON events
acp-cli claude "what is 2+2?" --format quiet  # final text only

# Timeout
acp-cli claude "large refactor task" --timeout 120
```

## Commands

```bash
acp-cli init                                    # interactive setup
acp-cli [agent] [prompt...]                     # persistent session prompt
acp-cli [agent] exec [prompt...]                # one-shot (no persistence)
acp-cli [agent] sessions new [--name <name>]    # create named session
acp-cli [agent] sessions list                   # list sessions
acp-cli [agent] sessions show                   # session details
acp-cli [agent] sessions close                  # close session
acp-cli [agent] sessions history                # conversation log
acp-cli [agent] cancel                          # cancel running prompt
acp-cli [agent] status                          # check session state
acp-cli [agent] set-mode <mode>                 # change agent mode
acp-cli [agent] set <key> <value>               # set session config
acp-cli config show                             # print merged config
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-s, --session <name>` | | Named session |
| `--approve-all` | | Auto-approve all tool calls |
| `--approve-reads` | **default** | Approve read-only tools, deny writes |
| `--deny-all` | | Deny all tool calls |
| `--cwd <dir>` | `.` | Working directory |
| `--format text\|json\|quiet` | `text` | Output format |
| `--timeout <seconds>` | | Max wait time |
| `-f, --file <path>` | | Read prompt from file (`-` for stdin) |
| `--no-wait` | | Fire-and-forget (queue and return) |
| `--agent-override <cmd>` | | Raw ACP command override |

## Config

Run `acp-cli init` or create `~/.acp-cli/config.json` manually:

```json
{
  "default_agent": "claude",
  "default_permissions": "approve_reads",
  "timeout": 60,
  "auth_token": "sk-ant-...",
  "agents": {
    "my-agent": {
      "command": "./custom-agent",
      "args": ["--flag"]
    }
  }
}
```

Project-level config: `.acp-cli.json` in git root (same format, overrides global).

### Auth Token Resolution

Token for Claude is resolved in order:

1. `ANTHROPIC_AUTH_TOKEN` environment variable
2. `~/.acp-cli/config.json` → `auth_token`
3. `~/.claude.json` → `oauthAccount.accessToken`
4. macOS Keychain (`Claude Code` service)

## Supported Agents

| Agent | Command | Type |
|-------|---------|------|
| claude | `npx @zed-industries/claude-agent-acp` | npm |
| codex | `npx @zed-industries/codex-acp` | npm |
| gemini | `gemini --acp` | native |
| copilot | `copilot --acp --stdio` | native |
| cursor | `cursor-agent acp` | native |
| goose | `goose acp` | native |
| kiro | `kiro-cli acp` | native |
| pi | `npx pi-acp` | npm |
| openclaw | `openclaw acp` | native |
| opencode | `npx opencode-ai acp` | npm |
| kilocode | `npx @kilocode/cli acp` | npm |
| kimi | `kimi acp` | native |
| qwen | `qwen --acp` | native |
| droid | `droid exec --output-format acp` | native |

Unknown agent names are treated as raw commands.

## Sessions

Sessions auto-resume by matching `(agent, git_root, session_name)`.

```bash
acp-cli claude sessions new --name api   # create named session
acp-cli claude -s api "add endpoint"     # use named session
acp-cli claude sessions list             # list all sessions
acp-cli claude sessions history          # view conversation log
```

The first `acp-cli` process for a session becomes the **queue owner** (holds the agent connection). Subsequent processes connect as queue clients via Unix socket.

## Architecture

```
Main thread (Send)          ACP thread (!Send, LocalSet)
├── CLI parsing             ├── AcpConnection (spawn_local)
├── Output rendering        ├── ClientSideConnection I/O
├── Permission resolution   └── BridgedAcpClient callbacks
├── Signal handling
└── Queue IPC server        Channel bridge (mpsc + oneshot)
```

## Library Usage

acp-cli can be used as a Rust library:

```rust
use acp_cli::bridge::AcpBridge;
use std::path::PathBuf;

let bridge = AcpBridge::start(
    "npx".to_string(),
    vec!["-y".into(), "@zed-industries/claude-agent-acp".into()],
    PathBuf::from("."),
).await?;

let result = bridge.prompt(vec!["fix the bug".into()]).await?;
bridge.shutdown().await?;
```

## License

MIT
