# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- **`--prompt-retries <n>`**: automatically retry transient prompt failures (connection errors, agent spawn failures, bridge channel closure) with exponential backoff and jitter. Default is `0` (no retry). Retries are guarded against side-effects: only connection-level errors (before any agent output is produced) trigger a retry. Semantic errors (permission denied, session not found, auth failures) fail immediately without retrying.

## [0.2.2] - 2026-03-25

### Fixed
- **ACP child process cleanup and reaping**: bridge shutdown now explicitly reaps child processes (`try_wait` + `start_kill` + `wait`) to prevent zombie process accumulation in long-running orchestrators.
- **Early error-path cleanup**: child process reaping now runs even when ACP initialization/session setup fails before the command loop starts.

### Tests
- Added bridge test coverage for initialization-failure cleanup/reaping path.
- Updated child cleanup tests to use cross-platform command invocation instead of Unix-only `sh`.

## [0.2.1] - 2026-03-23

### Fixed
- **OAuth token injection causes 401** (#22): OAuth tokens (`sk-ant-oat01-*`) are no longer injected via `ANTHROPIC_AUTH_TOKEN` env var. The Claude Agent SDK's env-var auth path omits the required `anthropic-beta: oauth-2025-04-20` header, causing authentication failure. OAuth tokens are now left for the SDK to resolve from macOS Keychain internally. Non-OAuth tokens (API keys) are still injected normally.

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
