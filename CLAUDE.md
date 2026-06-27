# CLAUDE.md — qingbird

## △ V0.2.14 — 状态机 + 多 Provider + 多轮对话 + i18n

### 关键路径

| 项 | 值 |
|--- |--- |
| **版本** | V0.2.14 |
| **架构** | 5-crate workspace: models → infra → tools → agents → binary |
| **二进制** | `qingbird` (crates/qbird-code) |
| **LLM** | DeepSeek / Ollama / OpenAI / Anthropic (5 种路由) |
| **CLI** | `--execute`, `--interactive`, `--provider`, `--model`, `--temperature` |
| **测试** | 112 tests（111 单元 + 1 集成 mock）|

### 4 门禁

```bash
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

### 文件结构

- `crates/qbird-code-models/` — Message / EflowError / RiskLevel / ToolCall
- `crates/qbird-code-infra/` — 5 Provider + config + http_client + env
- `crates/qbird-code-tools/` — 4 工具 (read_file/write_file/search_code/execute_command)
- `crates/qbird-code-agents/` — ReactLoop 状态机 + doom_loop + nudge
- `crates/qbird-code/` — 二进制入口，CLI + 交互模式
- `locales/` — 中英双语 i18n

### 关键约定

- 所有用户面向字符串走 `t!()`（zh-CN + en-US）
- 代码注释 / tracing 日志保持英文
- PR 前过 4 门禁
- 提交用 Conventional Commits
