---
name: acp-cli
description: Help developers use acp-cli — the Rust ACP CLI client for talking to coding agents (Claude, Codex, Gemini, etc.) over the Agent Client Protocol. Use when user asks about acp-cli commands, agent registry, session management, queue system, permission modes, or output formats.
---

# acp-cli

Headless CLI client for the Agent Client Protocol (ACP) — Rust port of ACPX.

14 built-in agents. 3 output formats. 3 permission modes. Queue system with Unix socket IPC.

## Quick Reference

```bash
acp-cli claude "fix the bug" --approve-all       # prompt
acp-cli claude exec "explain this"               # one-shot
acp-cli claude -f prompt.md                      # from file
echo "fix it" | acp-cli claude                   # stdin pipe
acp-cli claude --no-wait "refactor this"          # fire-and-forget
acp-cli claude sessions history                  # conversation log
acp-cli claude cancel                            # stop prompt
acp-cli claude status                            # check state
acp-cli claude set-mode plan                     # change mode
```

## When to Read References

| Task | File |
|------|------|
| Full command reference, all flags, exit codes | `references/cli-reference.md` |
| Agent registry, custom agents, config overrides | `references/agents.md` |
| Session management, scoping rules, queue system | `references/sessions.md` |
| Release process, version bump, CI publish | `references/release.md` |

## Key Design

- **Architecture**: Multi-threaded tokio + `spawn_blocking` bridge for `!Send` ACP futures
- **Queue**: First process = owner (holds connection), subsequent = clients (Unix socket IPC)
- **Sessions**: Keyed by `SHA-256(agent + dir + name)`, auto-resume from git root
- **Permissions**: `--approve-reads` default (Read/Glob/Grep/WebSearch/WebFetch/LSP allowed, writes denied)
