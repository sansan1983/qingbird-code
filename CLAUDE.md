# CLAUDE.md — qingbird

## 当前状态（会话交接）

| 项 | 值 |
|--- |--- |
| **上次会话** | 2026-06-30 |
| **已完成** | milestone/v0.3.1：agent 身份定位重写（system_prompt + nudge + sample profile）+ Subagent 重建（PR-A ×5 + PR-B1 ×5 + PR-B2 ×6 = 16 commits）；SubagentExecutor 真实实现 + delegate_task 工具 + SessionStore side-session 持久化；4 门禁全过 |
| **下一步** | v0.3.1 已完成，准备合并到 main（maintainer PR），之后开 v0.4.0 进化系统（CompactionManager / MemoryZone / Reflection Engine） |
| **当前版本** | 0.3.1 |
| **已修复** | api_key 空字符串回退、ToolCall.type 字段缺失、工具输出显示、yaml 模型名更新、stale provider_planned i18n key、hardcoded English string、Subagent 孤儿代码清理、SubagentExecutor 类型级 async 递归用 Box::pin 打破、ReactLoopConfig 因 SubagentExecutor 不 impl Debug 去掉 derive(Debug) |
| **备注** | PR-B2 完成：`docs/superpowers/plans/2026-06-29-agent-identity-and-subagent-rebuild.md`；milestone/v0.3.1 分支 16 commits，agent 身份 + subagent 体系写入；subagent 共享主 registry 安全设置但不递归暴露 delegate_task |

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
- `crates/qbird-code-agents/` — ReactLoop 状态机 + doom_loop + nudge + subagent (SubagentProfile + SubagentExecutor + delegate_task)
- `crates/qbird-code/` — 二进制入口，CLI + 交互模式 + REPL
- `locales/` — 中英双语 i18n
- `docs/` — CLI / Configuration / Profiles 用户文档（v0.3.0 新增），Subagent 章节（v0.3.1 新增）

### 关键约定

- 所有用户面向字符串走 `t!()`（zh-CN + en-US）
- 代码注释 / tracing 日志保持英文
- PR 前过 4 门禁
- 提交用 Conventional Commits，scope = 模块名
- 共享 workspace 版本号（0.2.x / 0.3.x 节奏）
