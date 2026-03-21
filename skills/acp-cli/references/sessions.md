# Session Management

## Session Scoping

Session unique key: `SHA-256("agent\0directory\0name")`.

Lookup:
1. Walk from `--cwd` up to git root (if no git repo, use cwd as-is)
2. Search `~/.acp-cli/sessions/` for matching open session
3. `name: None` matches only unnamed sessions
4. If no match, create new session

## Persistence

```
~/.acp-cli/
├── config.json              # global config
└── sessions/
    ├── <key>.json           # SessionRecord
    ├── <key>.log.jsonl      # conversation history (append-only)
    ├── <key>.pid            # PID of running acp-cli process
    ├── <key>.lease          # queue owner lease (PID + heartbeat)
    └── <key-12chars>.sock   # Unix socket for queue IPC
```

## Session Resume

Subsequent prompts from the same directory reuse the session key:
- Same conversation history file (appended)
- New ACP session ID (agents don't persist across process restarts)
- `sessions history` shows all prompts across runs

## Queue System

### Queue Owner
First `acp-cli` process for a session becomes the owner:
- Holds the AcpBridge (agent connection)
- Listens on Unix socket
- Executes prompts sequentially (FIFO, max queue depth: 16)
- Heartbeat every 5s (updates lease file)
- Shuts down after TTL expires with empty queue (default: 300s)

### Queue Client
Subsequent processes connect via Unix socket:
- Send prompt → receive `Queued { position }` → stream events → receive result
- Transparent to user — same output as direct execution

### Lease File
```json
{"pid": 12345, "start_time": 1711000000, "last_heartbeat": 1711000100}
```
Valid if: `now - last_heartbeat < TTL` AND process is alive (kill(pid, 0)).

### --no-wait
Queue prompt and return immediately:
```bash
acp-cli claude --no-wait "refactor the auth module"
# Prints: "Prompt queued (position 1)"
# Returns exit 0 immediately
```
Requires an active queue owner.

## Config

### Global: `~/.acp-cli/config.json`
```json
{
  "default_agent": "claude",
  "default_permissions": "approve_reads",
  "timeout": 60,
  "format": "text",
  "agents": {}
}
```

### Project: `.acp-cli.json` (in git root)
Same format. Project overrides global. CLI flags override both.
