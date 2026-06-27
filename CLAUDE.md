# CLAUDE.md — qingbird

## △ V0.2.0 — Workspace + ReAct + 多 Provider

版本 0.2.0，5-crate workspace + ReAct 循环 + 多 Provider。

### 关键路径

| 项 | 值 |
|--- |--- |
| **当前** | V0.2.0 稳定。死代码清理完成（删 `src/`、`tests/`、`src.v0.1.bak/`） |
| **架构** | 5-crate workspace: qbird-code-models / infra / tools / agents / qbird-code |
| **二进制** | `qingbird` (crates/qbird-code) |
| **LLM** | DeepSeek / Ollama / OpenAI / Anthropic |
| **测试** | 67 tests, 12 suites |

### 4 门禁

- `cargo build` — 编译通过
- `cargo clippy --all-targets -- -D warnings` — 零告警
- `cargo fmt --check` — 格式一致
- `cargo test` — 全通过

### 文件结构

- `crates/qbird-code-models/` — 核心类型 (EflowError/Message/ProviderKind 等)
- `crates/qbird-code-infra/` — 4 Provider + config + event + env + http_client
- `crates/qbird-code-tools/` — 工具定义与注册（读/写/搜索/执行命令）
- `crates/qbird-code-agents/` — ReAct 循环 + Subagent + 死循环检测 + Nudge 机制
- `crates/qbird-code/` — 二进制入口，CLI (--execute / --interactive)
- `locales/` — 中英双语

## 收工仪式

修改后更新上面「当前」状态表 + 把旧日志行移到 WORKLOG.md。
