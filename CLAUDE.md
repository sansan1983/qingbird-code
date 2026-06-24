# CLAUDE.md — qingbird

## △ V0.1.0 — DeepSeek 专属重构

版本 0.1.0，DeepSeek-only。所有多 Provider 抽象(`ProtocolKind`/`ProviderConfig`/`ModelEntry`/`tier` 路由)已删除。

### 关键路径

| 项 | 值 |
|--- |--- |
| **当前** | V0.1.0 restructuring（eflow → qingbird 改名 + 瘦身） |
| **版本** | 0.1.0 |
| **LLM** | DeepSeek 唯一，配置在 `~/.qingbird/config.yaml` 或 `qingbird.yaml` |
| **二进制** | `qingbird` |
| **库** | `qingbird_code` |

### 4 门禁

- `cargo build` — 编译通过
- `cargo clippy --all-targets -- -D warnings` — 零告警
- `cargo fmt --check` — 格式一致
- `cargo test` — 全通过

### 文件结构

- `src/main.rs` — CLI 入口，`--execute`/子命令/TUI 路由
- `src/infrastructure/llm/` — DeepSeek provider、L1/L2 缓存
- `src/application/` — Concierge + Orchestrator
- `src/capability/` — Decisioner/Executor/Feedbacker + Subagent 池 + Tools
- `locales/` — 中英双语

## 收工仪式

修改后更新上面「当前」状态表 + 把旧日志行移到 WORKLOG.md。
