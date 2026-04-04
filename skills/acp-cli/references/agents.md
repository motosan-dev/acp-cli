# Agent Registry

## Built-in Agents

| Name | Command | Args |
|------|---------|------|
| claude | `npx` | `-y @agentclientprotocol/claude-agent-acp@^0.24.2` |
| codex | `npx` | `-y @zed-industries/codex-acp@^0.10.0` |
| gemini | `gemini` | `--acp` |
| copilot | `copilot` | `--acp --stdio` |
| cursor | `cursor-agent` | `acp` |
| goose | `goose` | `acp` |
| kiro | `kiro-cli-chat` | `acp` |
| pi | `npx` | `-y pi-acp@^0.0.22` |
| openclaw | `openclaw` | `acp` |
| opencode | `npx` | `-y opencode-ai acp` |
| kimi | `kimi` | `acp` |
| qwen | `qwen` | `--acp` |
| droid | `droid` | `exec --output-format acp` |
| kilocode | `npx` | `-y @kilocode/cli acp` |
| iflow | `iflow` | `--experimental-acp` |
| qoder | `qodercli` | `--acp` |
| trae | `traecli` | `acp serve` |

## Resolution Order

1. Config overrides (`agents` field in config)
2. Built-in registry
3. Raw command fallback (unknown names treated as commands)

## Custom Agents

### Via config file

```json
{
  "agents": {
    "my-agent": {
      "command": "./custom-agent",
      "args": ["--acp", "--verbose"]
    }
  }
}
```

### Via CLI flag

```bash
acp-cli --agent-override "./my-agent --flag" "prompt text"
```

## Agent Aliases

Agent names are case-insensitive and trimmed.
