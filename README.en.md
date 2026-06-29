# qingbird (青鸟)

> Efficient Rust Agent collaboration framework — single binary, multi-provider, ReAct loop

[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-orange)]()

**[简体中文](README.md)**

## Quick Start

```bash
# 1. Build
cargo build --release

# 2. Set API key
export DEEPSEEK_API_KEY="sk-..."

# 3. Run
./target/release/qingbird --execute "Analyze the current directory"
```

## Configuration

Config file: `qingbird.yaml` (current dir) or `~/.qingbird/config.yaml`

```yaml
llm:
  active: deepseek                    # default provider
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"    # env var substitution supported
    base_url: "https://api.deepseek.com"
    default_model: "deepseek-chat"
    thinking_enabled: true
    thinking_effort: "high"
    timeout_secs: 30
    max_retries: 3
    retry_backoff_ms: 1000
```

Works with just the environment variable (`DEEPSEEK_API_KEY` / `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`), no config file needed.

## CLI Usage

```
qingbird --execute "prompt"                           Single execution
qingbird --interactive                                 Interactive REPL
qingbird --provider ollama --execute "..."             Switch provider on the fly
qingbird --model deepseek-chat --execute "..."         Switch model on the fly
qingbird --temperature 0.3 --execute "..."             Set temperature on the fly
qingbird --help                                        All options
```

## Interactive Mode

Enter `--interactive` for multi-turn conversation. Slash commands:

```
/help                     Show help
/model <name>             Switch model
/temperature <n>          Set temperature (0.0–2.0)
/usage                    Show token usage
/sessions                 List saved sessions
/session load <id>        Load a previous session
/sdd run <input>          Run SDD workflow
/quit /exit               Exit
```

Conversation history auto-truncates at 50 messages (keeps system + recent half).

## Supported Providers

| Provider | `active` value | Env var |
|----------|---------------|---------|
| DeepSeek (OpenAI protocol) | `deepseek` | `DEEPSEEK_API_KEY` |
| DeepSeek (Anthropic protocol) | `deepseek-anthropic` | `DEEPSEEK_API_KEY` |
| Ollama (local) | `ollama` | none |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |

```bash
# Example: local Ollama
qingbird --provider ollama --interactive

# Example: GPT-4o
export OPENAI_API_KEY="sk-..."
qingbird --provider openai --model gpt-4o --execute "Hello"
```

## Install

```bash
# From source
git clone <repo>
cd qingbird-code
cargo build --release
./target/release/qingbird --execute "..."

# Or cargo install
cargo install qingbird-code
```

## Architecture

```
qingbird (binary CLI)
  └── qbird-code-agents    — ReactLoop + doom loop detection + nudge
  └── qbird-code-tools     — 7 built-in tools (read/write/search/command/glob/list_dir/web_fetch)
  └── qbird-code-infra     — 5 LLM providers + HTTP client + memory + config
  └── qbird-code-models    — core types (Message/Error/RiskLevel)
```

**Strict dependency direction**: lower layers must not import upper layers.

## Documentation

| Doc | Description |
|-----|-------------|
| [CLI Reference](docs/cli.md) | All startup flags + interactive slash commands |
| [Configuration Reference](docs/configuration.md) | All `qingbird.yaml` fields, defaults & validation |
| [Profile Guide](docs/profiles.md) | Create, use, and switch user profiles |

## License

MIT / Apache-2.0 dual-licensed.
