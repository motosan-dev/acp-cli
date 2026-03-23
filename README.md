# acp-cli

Headless CLI client for the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/). Rust port of [ACPX](https://github.com/openclaw/acpx).

Talk to coding agents (Claude, Codex, Gemini, etc.) over a structured protocol instead of terminal scraping.

## Install

```bash
cargo install acp-cli
```

## Setup

```bash
acp-cli init
```

Detects Claude Code installation, finds existing auth tokens, and writes `~/.acp-cli/config.json`.

## Usage

```bash
# Simple prompt (uses claude by default)
acp-cli claude "fix the auth bug"

# One-shot mode (no session persistence)
acp-cli claude exec "what does this function do?"

# JSON output for automation
acp-cli claude "list all TODOs" --format json

# Quiet mode (final text only)
acp-cli claude "what is 2+2?" --format quiet --approve-all

# Use a different agent
acp-cli codex "refactor this module"
acp-cli gemini "explain this error"

# Named sessions for parallel work
acp-cli claude -s backend "fix the API"
acp-cli claude -s frontend "update the UI"

# Custom timeout
acp-cli claude "large refactor task" --timeout 120
```

## Supported Agents

| Agent | Command |
|-------|---------|
| claude | `npx @zed-industries/claude-agent-acp` |
| codex | `npx @zed-industries/codex-acp` |
| gemini | `gemini --acp` |
| copilot | `copilot --acp --stdio` |
| cursor | `cursor-agent acp` |
| goose | `goose acp` |
| kiro | `kiro-cli acp` |
| pi | `npx pi-acp` |
| openclaw | `openclaw acp` |
| opencode | `npx opencode-ai acp` |
| kimi | `kimi acp` |
| qwen | `qwen --acp` |
| droid | `droid exec --output-format acp` |

Unknown agent names are treated as raw commands.

## Permission Modes

| Flag | Behavior |
|------|----------|
| `--approve-all` | Auto-approve all tool calls |
| `--approve-reads` | Approve read-only tools, deny writes (default) |
| `--deny-all` | Deny all tool calls |

## Output Formats

- **text** (default) — streaming text with tool status
- **json** — NDJSON, one event per line
- **quiet** — final text only

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

### Auth Token Resolution

The auth token for Claude is resolved in order:

1. `ANTHROPIC_AUTH_TOKEN` environment variable
2. `~/.acp-cli/config.json` → `auth_token`
3. `~/.claude.json` → `oauthAccount.accessToken`
4. macOS Keychain (`Claude Code` service)

## Session Management

```bash
acp-cli claude sessions new              # create new session
acp-cli claude sessions new --name api   # named session
acp-cli claude sessions list             # list sessions
```

Sessions auto-resume by matching `(agent, git_root, name)`.

## License

MIT
