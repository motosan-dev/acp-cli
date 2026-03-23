# acp-cli

Headless CLI client for the Agent Client Protocol (ACP). Rust port of ACPX.

## Commands

```bash
cargo fmt                    # format
cargo clippy -- -D warnings  # lint
cargo test                   # test (needs LIBRARY_PATH on macOS, see below)
cargo check                  # fast compile check
```

macOS linker fix (libiconv):
```bash
LIBRARY_PATH="/opt/homebrew/opt/libiconv/lib" cargo test
```

## Source Structure

```
src/
├── main.rs              # CLI entry point, clap parsing, command dispatch
├── lib.rs               # Public library API (re-exports)
├── config.rs            # AcpCliConfig — global + project config loading/merging
├── error.rs             # AcpCliError enum (thiserror)
├── agent/
│   └── registry.rs      # Agent name → command resolution (14 built-in agents)
├── bridge/
│   ├── mod.rs           # AcpBridge — spawn_blocking + LocalSet bridge to !Send ACP futures
│   ├── commands.rs      # BridgeCommand enum (Prompt, Cancel, SetMode, SetConfig, Shutdown)
│   └── events.rs        # BridgeEvent enum (TextChunk, ToolUse, PermissionRequest, etc.)
├── cli/
│   ├── mod.rs           # Cli struct (clap), Commands/SessionAction/ConfigAction enums
│   ├── init.rs          # `acp-cli init` interactive setup
│   ├── prompt.rs        # run_prompt() (persistent session) + run_exec() (one-shot)
│   ├── prompt_source.rs # Resolve prompt from args, file, or stdin
│   └── session.rs       # Session subcommands (new, list, show, close, history, cancel, status)
├── client/
│   ├── mod.rs           # BridgedAcpClient — ACP client handler (tool use, permissions)
│   └── permissions.rs   # PermissionMode (ApproveAll, ApproveReads, DenyAll)
├── output/
│   ├── mod.rs           # OutputRenderer trait
│   ├── text.rs          # TextRenderer (streaming + spinner)
│   ├── json.rs          # JsonRenderer (NDJSON events)
│   └── quiet.rs         # QuietRenderer (final text only)
├── queue/
│   ├── mod.rs           # Queue system overview
│   ├── owner.rs         # QueueOwner — holds agent connection, IPC server
│   ├── client.rs        # QueueClient — connects to owner via Unix socket
│   ├── ipc.rs           # IPC protocol (Unix socket)
│   ├── lease.rs         # Lease/heartbeat management
│   └── messages.rs      # IPC message types
└── session/
    ├── mod.rs
    ├── scoping.rs       # Session key: SHA-256(agent + dir + name), git root resolution
    ├── persistence.rs   # Session record JSON (create, load, update, close)
    ├── history.rs       # Conversation history (JSONL append)
    └── pid.rs           # PID file management
```

## Key Architecture

**AcpBridge** is the core abstraction. ACP SDK uses `Rc`/`spawn_local` (!Send), so the bridge runs in `spawn_blocking` + `LocalSet` on a dedicated thread, communicating with the main thread via mpsc/oneshot channels.

```
Main thread (Send)          ACP thread (!Send, LocalSet)
├── CLI / Output            ├── Child process (stdin/stdout pipes)
├── Permission handling     ├── ClientSideConnection
└── Queue IPC server        └── BridgedAcpClient callbacks
         ↕ mpsc channels ↕
```

**Auth token resolution order:**
1. `ANTHROPIC_AUTH_TOKEN` env var
2. `~/.acp-cli/config.json` → `auth_token`
3. `~/.claude.json` → `oauthAccount.accessToken`
4. macOS Keychain

**Config merge order:** global (`~/.acp-cli/config.json`) → project (`.acp-cli.json`) → CLI flags.

## Rules

- Always use `cargo fmt` before committing
- Never use `git add -A` — add specific files
- Library code lives in `src/lib.rs` + modules; `main.rs` is only CLI entry
- Errors use `thiserror` via `AcpCliError`; return `Result<T, AcpCliError>`
- New CLI commands: add variant to `Commands` enum in `cli/mod.rs`, handle in `main.rs`
- New agents: add to `default_registry()` in `agent/registry.rs`
