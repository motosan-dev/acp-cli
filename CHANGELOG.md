# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.2.0] - 2026-03-23

### Added
- `acp-cli init` interactive setup command — detects Claude Code, finds auth tokens, writes config
- `auth_token` field in config — persist Anthropic auth token in `~/.acp-cli/config.json`
- Auth token resolution chain: env var → config → `~/.claude.json` → macOS Keychain
- CLAUDE.md, AGENTS.md project documentation
- CLI integration tests (70 total tests)

## [0.1.0] - 2026-03-21

### Added
- Core ACP CLI client with multi-threaded bridge architecture
- Agent registry with 14 built-in agents (Claude, Codex, Gemini, Copilot, Cursor, Goose, Kiro, Pi, OpenClaw, OpenCode, Kimi, Qwen, Droid, Kilocode)
- Three output formats: text (with spinner), JSON (NDJSON), quiet
- Three permission modes: approve-all, approve-reads (default), deny-all
- Session management: new, list, show, close, history
- Session resume with conversation continuity
- Conversation history logging (JSONL)
- Prompt from file (`-f`) and stdin pipe support
- Project-level config (`.acp-cli.json`)
- Global config (`~/.acp-cli/config.json`)
- Cancel and status commands with PID tracking
- Signal handling (Ctrl+C graceful cancel)
- Timeout support (`--timeout`)
- Queue system with Unix socket IPC
- Queue owner process with lease/heartbeat management
- Queue client for connecting to existing owner
- Fire-and-forget mode (`--no-wait`)
- `set-mode` and `set` commands for runtime config
