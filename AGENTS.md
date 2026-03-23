# AGENTS.md — acp-cli Development Brief

Read this before writing any code.

---

## Project Overview

**acp-cli** is a headless CLI client for the Agent Client Protocol (ACP). It spawns coding agents (Claude, Codex, Gemini, etc.) as child processes and communicates over structured JSON-RPC via stdio pipes.

- Version: 0.2.1
- Language: Rust (edition 2024)
- Published to: crates.io (`acp-cli`)
- GitHub: https://github.com/motosan-dev/acp-cli

---

## Repository Structure

```
acp-cli/
├── Cargo.toml
├── src/
│   ├── main.rs            # CLI entry point
│   ├── lib.rs             # Library re-exports
│   ├── config.rs          # Config loading + merging
│   ├── error.rs           # Error types (thiserror)
│   ├── agent/registry.rs  # Agent name → command mapping
│   ├── bridge/            # AcpBridge (spawn_blocking + LocalSet)
│   ├── cli/               # Commands: init, prompt, exec, sessions
│   ├── client/            # BridgedAcpClient + permission resolution
│   ├── output/            # Renderers: text, json, quiet
│   ├── queue/             # Queue owner/client, IPC, lease
│   └── session/           # Session scoping, persistence, history
├── skills/acp-cli/        # AI skill card
├── README.md
├── llms.txt               # LLM-consumable API reference
├── CLAUDE.md              # Claude Code project instructions
├── CHANGELOG.md
└── LICENSE
```

---

## Key Design Decisions

- **!Send bridge pattern** — ACP SDK uses `Rc`/`spawn_local`; bridge runs in `spawn_blocking` + `LocalSet`, communicates via mpsc channels
- **Queue system** — first process becomes queue owner (holds agent connection), subsequent processes connect as clients via Unix socket
- **Session scoping** — key is `SHA-256(agent + "\0" + git_root + "\0" + name)`, ensuring isolation per project/agent/name
- **Auth token chain** — env var → config file → `~/.claude.json` → macOS Keychain (never guess, always explicit)
- **Permission modes** — three modes (approve-all, approve-reads, deny-all) applied at BridgedAcpClient level
- **Agent registry** — hardcoded defaults + config overrides + raw command fallback

---

## Coding Standards

- `cargo fmt` before every commit
- `cargo clippy -- -D warnings` — zero warnings policy
- Errors: use `thiserror` via `AcpCliError`; never `unwrap()` in library code
- Async: `tokio` runtime; `async-trait` for trait objects
- Serialization: `serde` with `#[serde(default)]` for backward-compatible config fields
- Tests: `#[tokio::test]` for async tests

---

## Common Commands

```bash
# Development
cargo fmt
cargo clippy -- -D warnings
cargo check

# Testing (macOS needs libiconv)
LIBRARY_PATH="/opt/homebrew/opt/libiconv/lib" cargo test

# Run locally
LIBRARY_PATH="/opt/homebrew/opt/libiconv/lib" cargo run -- claude "hello"
```

---

## Adding a New Agent

1. Add entry to `default_registry()` in `src/agent/registry.rs`
2. Update agent table in `README.md` and `llms.txt`
3. No code changes needed — registry maps name → command + args

---

## Adding a New CLI Command

1. Add variant to `Commands` enum in `src/cli/mod.rs`
2. Add match arm in `src/main.rs` `run()` function
3. Implement in `src/cli/<command>.rs`
4. Update `Commands` section in `README.md`, `llms.txt`

---

## What NOT to Do

- Do not use `unwrap()` or `expect()` in library code (only in main.rs after error handling)
- Do not add agent-specific logic outside `agent/registry.rs` and `bridge/mod.rs`
- Do not store secrets in config without the user's explicit consent (`init` asks)
- Do not break the `AcpBridge` public API — it's used as a library by `motosan-workflow-core`
- Do not use `git add -A` — always add specific files

---

## Before Committing

```bash
cargo fmt
cargo clippy -- -D warnings
LIBRARY_PATH="/opt/homebrew/opt/libiconv/lib" cargo check
```

---

## Releasing

```bash
# 1. Update CHANGELOG.md (move Unreleased → version)
# 2. Bump version in Cargo.toml
# 3. Update version in: llms.txt, AGENTS.md
# 4. Commit + tag + push
git commit -m "chore: release v0.2.0"
git tag -a v0.2.0 -m "v0.2.0 — summary"
git push origin main v0.2.0
```

Tag push triggers `publish.yml` → crates.io.
