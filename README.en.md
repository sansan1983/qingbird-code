# eflow

> **Efficient Flow** — A multi-layer Agent collaboration framework written in Rust
> *One command to rule them all.*

[![Status](https://img.shields.io/badge/status-v1.3.3%20released-brightgreen)]()
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-orange)]()
[![Tests](https://img.shields.io/badge/tests-312%20passed-blue)]()

**[简体中文](README.md)**

eflow is a multi-layer Agent collaboration framework written in Rust, with **zero-blocking dialogue** as its
first design principle. Through **industry-identity-driven SOP dispatch**, **hierarchical decision execution**,
and **intelligent context & memory management**, eflow makes AI work like a well-trained team.

### Core Features

- **Zero-blocking dialogue** — Concierge dispatches and returns immediately; tasks run async, progress via event channel
- **Hierarchical decisions** — Decisioner → Executor → Feedbacker pipeline; rule + LLM dual-driven
- **Multi-provider LLM** — Configure any OpenAI/Anthropic-compatible provider via `~/.eflow/providers/*.yaml`; tier routing + rate-limit degradation
- **Three-tier memory** — Working (in-memory LRU) → Project (SQLite FTS5) → User (SQLite FTS5)
- **i18n** — Built-in zh-CN / en-US, based on rust-i18n
- **Zero-dep deploy** — Single Rust binary; Windows / Linux / macOS
- **Headless mode** — `eflow session start` with NDJSON stdio contract (for v2.0 GUI frontends)

### Quick Start

#### Prerequisites

- Rust 2024 edition (stable ≥ 1.85)
- API key: Anthropic or OpenAI

#### Install

```bash
git clone https://github.com/sansan1983/eflow.git
cd eflow
cargo build --release
```

#### Configure

Create `eflow.yaml` (v1.3 schema — providers in `~/.eflow/providers/*.yaml`):

```yaml
core:
  language: zh-CN
  timezone: UTC

llm:
  # v1.3+: routing references ~/.eflow/providers/<id>.yaml id
  routing:
    strong: anthropic
    medium: anthropic
    light: anthropic
  cache:
    l1_enabled: true

memory:
  working_memory_limit: 100
  project_db_path: ./data/project.db
  user_db_path: ./data/user.db
  cleanup_interval_hours: 24

security:
  risk_threshold: L2
  allowed_paths: []

profiles:
  default: developer
  available: [developer]
```

And create `~/.eflow/providers/anthropic.yaml`:

```yaml
id: anthropic
display_name: Anthropic
protocol: anthropic_compatible
base_url: "https://api.anthropic.com"
api_key: "${ANTHROPIC_API_KEY}"
default_model: "claude-sonnet-4-6"
```

Or run `eflow init` to launch the setup wizard.

#### Run

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/eflow --execute "Read Cargo.toml and summarize the project"
```

### Architecture

```
Interaction  →  TUI (ratatui) + CLI (--execute) + Headless (eflow session start, NDJSON contract)
Orchestration →  Concierge (zero-blocking) → Orchestrator (decompose + schedule, layer-parallel)
Capability   →  Decisioner → Executor → Feedbacker (3-role pipeline) + Subagent pool
Infrastructure → LLM / Memory / Context / Event / Profile / Tools
```

Detailed architecture: [`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md)
(v1.0 original design; v1.3 LLM provider abstraction: [`v1.3-llm-abstract-design.md`](docs/superpowers/specs/2026-06-17-eflow-v1.3-llm-abstract-design.md))

### Project Status

| Milestone | Status |
|-----------|--------|
| v1.0 Core | ✅ Released (end-to-end runnable skeleton) |
| v1.1 L2 cache + Subagent pool | ✅ Released (M4.5 + M8 + M10.5) |
| v1.2 Debt cleanup + parallel dispatch + TUI | ✅ Released (D1-D4 + E1-E6 + F1-F6) |
| v1.3.0 LLM abstraction + provider yaml | ✅ Released (spec A — 26 tasks) |
| v1.3.1 Wizard + slash commands | ✅ Released (spec B1 — 12 tasks) |
| v1.3.2 CLI contract + headless | ✅ Released (spec B2 — 12 tasks) |
| v1.3.3 spec C retracted | ✅ Released (spec C — 9 tasks; 3-level abstraction retracted in PR #21) |
| v1.4 spec D rendering pipeline | 🔵 Planned (spec + plan docs merged to main, PR1 awaits remote server) |

### Documentation

- Architecture design: [`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md)
- Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md)
- Session handoff: [CLAUDE.md](CLAUDE.md)
- AI agent quick reference: [AGENTS.md](AGENTS.md)

### Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for branch strategy and development workflow.

> ⚠ **Important rule**: Since v1.1, all changes must go through `feature/*` or `fix/*` branches + PR merge. **Direct push to `main` is forbidden**.

### License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Credits

eflow is maintained by the eflow contributors.
