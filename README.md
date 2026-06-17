# eflow

> **Efficient Flow** — Rust 多层 Agent 协作框架
> *One command to rule them all.*

[![Status](https://img.shields.io/badge/status-v1.1%20released-brightgreen)]()
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-orange)]()
[![Tests](https://img.shields.io/badge/tests-12%20files-blue)]()

[English](#english) · [简体中文](#简体中文)

---

## 简体中文

eflow 是一款以 Rust 为核心的多层 Agent 协作框架，以 **零阻塞对话** 为第一设计原则，通过
**行业身份驱动的 SOP 调度**、**分层决策执行**、**智能上下文与记忆管理**，让 AI 真正像一支训练有素的
团队那样工作。

### 核心特性

| 特性 | 说明 |
|------|------|
| **零阻塞对话** | Concierge 入口派发即返回，任务异步执行，事件通道实时回显进度 |
| **分层决策** | Decisioner → Executor → Feedbacker 三角色管线，规则+LLM 双驱动 |
| **多 Provider LLM** | Anthropic + OpenAI 双 provider，tier 路由 + 限流降级 |
| **三层记忆** | Working (内存 LRU) → Project (SQLite FTS5) → User (SQLite FTS5) |
| **多语言** | 内置 zh-CN / en-US 双语，基于 rust-i18n |
| **零依赖部署** | Rust 编译为单文件可执行，Windows/Linux/macOS 全平台 |

### 快速开始

#### 前置要求

- Rust 2024 edition（stable ≥ 1.85）
- API Key：Anthropic 或 OpenAI 任一

#### 安装

```bash
git clone https://github.com/sansan1983/eflow.git
cd eflow
cargo build --release
```

#### 配置

创建 `eflow.yaml`：

```yaml
core:
  language: zh-CN
  timezone: UTC

llm:
  providers:
    anthropic:
      api_key: ${ANTHROPIC_API_KEY}
      default_model: claude-sonnet-4-6
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

#### 运行

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/eflow --execute "读取 Cargo.toml 总结项目结构"
```

### 架构

```
交互层       →  TUI (ratatui, v1.2) + CLI (--execute 单次模式)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+调度, v1.2 按层并行)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  LLM / Memory / Context / Event / Profile / Tools
```

详细架构见 [`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md)

### 项目状态

| 里程碑 | 状态 |
|--------|------|
| v1.0 内核 | ✅ 已发布（端到端可运行骨架） |
| v1.1 L2 缓存 + 多 Subagent 池 | ✅ 已发布（M4.5 + M8 + M10.5） |

### 文档

- 架构设计：[`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md)
- 贡献指南：[CONTRIBUTING.md](CONTRIBUTING.md)
- 变更日志：[CHANGELOG.md](CHANGELOG.md)
- 会话交接：[CLAUDE.md](CLAUDE.md)

### 贡献

欢迎贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解分支策略和开发流程。

> ⚠️ **重要规则**：v1.1 起所有改动必须通过 `feature/*` 或 `fix/*` 分支 + PR 合并，**禁止直接 push 到 main**。

### 许可证

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### 致谢

eflow 由 eflow contributors 共同维护。

---

## English

eflow is a multi-layer Agent collaboration framework written in Rust, with **zero-blocking dialogue** as its
first design principle. Through **industry-identity-driven SOP dispatch**, **hierarchical decision execution**,
and **intelligent context & memory management**, eflow makes AI work like a well-trained team.

### Core Features

- **Zero-blocking dialogue** — Concierge dispatches and returns immediately; tasks run async, progress via event channel
- **Hierarchical decisions** — Decisioner → Executor → Feedbacker pipeline; rule + LLM dual-driven
- **Multi-provider LLM** — Anthropic + OpenAI; tier routing + rate-limit degradation
- **Three-tier memory** — Working (in-memory LRU) → Project (SQLite FTS5) → User (SQLite FTS5)
- **i18n** — Built-in zh-CN / en-US, based on rust-i18n
- **Zero-dep deploy** — Single Rust binary; Windows / Linux / macOS

### Quick Start

```bash
git clone https://github.com/sansan1983/eflow.git
cd eflow
cargo build --release
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/eflow --execute "Read Cargo.toml and summarize the project"
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and contribution rules.

### License

Dual-licensed under MIT or Apache 2.0 at your option.
