# CLAUDE.md — qingbird

## △ V0.1.0 — DeepSeek 专属重构

版本 0.1.0，DeepSeek-only。所有多 Provider 抽象(`ProtocolKind`/`ProviderConfig`/`ModelEntry`/`tier` 路由)已删除。

### 关键路径

| 项 | 值 |
|--- |--- |
| **当前** | V0.1.0 restructuring 完成（13 tasks，全部门禁通过） |
| **上次完成** | V0.1.0 实施：eflow → qingbird 重命名、DeepSeek 专属瘦身、Provider 抽象全部删除、DeepSeek 真链路 smoke test ×3 通过。45 files / +1403 -3189。 |

**近期日志**：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-24 | V0.1.0 restructuring 完成 | 13 tasks 全部实施：项目重命名、删多 Provider 抽象（ProtocolKind/ProviderConfig/tier 路由等）、配置路径更新、config.yaml 简化、测试清理、ADR-0018、git bundle 归档。4 门禁全过。commit `09b020b` |
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
