# CLAUDE.md — qingbird

## 当前状态（会话交接）

| 项 | 值 |
|--- |--- |
| **上次会话** | 2026-06-28 |
| **已完成** | Phase 1 (v0.2.18) 全部 12 task 已合并 main；DeepSeek API 接通修复已推送 |
| **下一步** | Phase 2 (v0.2.19) — 从 19-04（保留类型实现）或 19-03（Streaming）开始 |
| **当前版本** | 0.2.18 |
| **已修复** | api_key 空字符串回退、ToolCall.type 字段缺失、工具输出显示、yaml 模型名更新 |
| **备注** | Phase 2 计划见 `docs/superpowers/plans/2026-06-27-qingbird-v0.3-implementation-plan.md` |

## △ V0.3.0 — 日常编码助手可用态

### 关键路径

| 项 | 值 |
|--- |--- |
| **目标版本** | V0.3.0（v0.2.18 清理 → v0.2.19 接线 → v0.3.0 打磨） |
| **架构** | 5-crate workspace: models → infra → tools → agents → binary |
| **二进制** | `qingbird` (crates/qbird-code) |
| **LLM** | DeepSeek / Ollama / OpenAI / Anthropic (5 种路由) |
| **CLI** | `--execute`, `--interactive`, `--provider`, `--model`, `--temperature`, `--lang`, `--profile` |
| **工具** | 7 内置 + 1 `edit`（v0.3.0 新增） |
| **斜杠命令** | 7 实际可用 + 4 计划中 |
| **Profile** | 用户级 `data_dir()/qingbird/profiles/*.yaml` |

### 4 门禁

```bash
cargo fmt --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace
cargo build
```

### 文件结构

- `crates/qbird-code-models/` — Message / EflowError / RiskLevel / ToolCall / PermissionSet / Role / Capability / MemoryCategory / Importance / RetryPolicy
- `crates/qbird-code-infra/` — 5 Provider + config + http_client + env + profile + stream_format
- `crates/qbird-code-tools/` — 8 工具 (read/write/search/command/glob/list_dir/web_fetch/edit)
- `crates/qbird-code-agents/` — ReactLoop 状态机 + doom_loop + nudge + subagent_pool
- `crates/qbird-code/` — 二进制入口，CLI + 交互模式 + REPL
- `locales/` — 中英双语 i18n
- `docs/` — CLI / Configuration / Profiles 用户文档（v0.3.0 新增）

### 关键约定

- 所有用户面向字符串走 `t!()`（zh-CN + en-US）
- 代码注释 / tracing 日志保持英文
- PR 前过 4 门禁
- 提交用 Conventional Commits，scope = 模块名
- 共享 workspace 版本号（0.2.x / 0.3.x 节奏）
