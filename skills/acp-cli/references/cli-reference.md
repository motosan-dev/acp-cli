# CLI Reference

## Commands

| Command | Description |
|---------|-------------|
| `acp-cli [agent] [prompt...]` | Persistent session prompt (default) |
| `acp-cli [agent] exec [prompt...]` | One-shot, no session persistence |
| `acp-cli [agent] sessions new [--name]` | Create named session |
| `acp-cli [agent] sessions list` | List sessions for cwd |
| `acp-cli [agent] sessions show` | Show session details |
| `acp-cli [agent] sessions close` | Soft-close session |
| `acp-cli [agent] sessions history` | Show conversation log |
| `acp-cli [agent] cancel` | Cancel running prompt (SIGTERM) |
| `acp-cli [agent] status` | Show session status (running/idle/closed) |
| `acp-cli [agent] set-mode <mode>` | Change agent session mode |
| `acp-cli [agent] set <key> <value>` | Set session config option |
| `acp-cli config show` | Print loaded config as JSON |

## Global Flags

| Flag | Default | Description |
|------|---------|-------------|
| `-s, --session <name>` | — | Named session |
| `--approve-all` | — | Auto-approve all tool calls |
| `--approve-reads` | default | Approve read-only tools only |
| `--deny-all` | — | Deny all tool calls |
| `--cwd <dir>` | `.` | Working directory |
| `--format text\|json\|quiet` | `text` | Output format |
| `--timeout <seconds>` | — | Max wait for prompt |
| `-f, --file <path>` | — | Read prompt from file (`-` for stdin) |
| `--no-wait` | — | Fire-and-forget mode |
| `--agent-override <cmd>` | — | Raw ACP command (bypass registry) |
| `--verbose` | — | Debug output to stderr |

## Exit Codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | Success | Prompt completed |
| 1 | AgentError | Agent/runtime/IO/connection error |
| 2 | UsageError | CLI argument error |
| 3 | Timeout | --timeout exceeded |
| 4 | NoSession | Session not found |
| 5 | PermissionDenied | All permissions denied |
| 130 | Interrupted | SIGINT (Ctrl+C) |

## Output Formats

### text (default)
Streaming text to stdout, spinner/status to stderr. No spinner when piped.

### json (NDJSON)
```jsonl
{"type":"session","sessionId":"abc-123"}
{"type":"text","content":"Hello"}
{"type":"tool","name":"Read"}
{"type":"done"}
```

### quiet
Final text only. No status, no session info.

## Signal Handling

1. First Ctrl+C → cooperative cancel (waits 3s)
2. Second Ctrl+C → force kill + exit 130

## Timeout

`--timeout N` wraps the entire prompt. On timeout: cancel → wait 3s → kill → exit 3.

## Permission Modes

| Mode | Read tools | Write tools |
|------|-----------|-------------|
| `--approve-all` | Allow | Allow |
| `--approve-reads` | Allow | Deny |
| `--deny-all` | Deny | Deny |

Read-only tools: Read, Glob, Grep, WebSearch, WebFetch, LSP.
