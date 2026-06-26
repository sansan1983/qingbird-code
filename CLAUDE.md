# CLAUDE.md — qingbird

## △ V0.2.0 — Workspace + ReAct + 多 Provider

版本 0.2.0，5-crate workspace + ReAct 循环 + 多 Provider。

### 关键路径

| 项 | 值 |
|--- |--- |
| **当前** | V0.2.0 实施完成 + 最终审查修复（21 tasks, 所有门禁通过, 68 tests） |
| **上次完成** | V0.2.0 实施：单crate D-E-F → 5-crate workspace ReAct 循环 + 多 Provider。新架构：models → infra → tools → agents → binary。DeepSeek 深度优化（双协议 + thinking mode + reasoning_content），Ollama 本地真链路测试，OpenAI/Anthropic 协议骨架占位。ReAct 循环（含死循环检测 + Nudge 机制 + 系统提示词 + i18n）替代 D-E-F 管线。 |

**近期日志**：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-26 | V0.2.0 实施完成 | 20 tasks: workspace 5-crate split + 4 Provider (DeepSeek双协议/Ollama/OpenAI占位/Anthropic占位) + ReAct 循环 (死循环检测 + Nudge + i18n) + Subagent + 集成、所有门禁通过 |
| 2026-06-27 | V0.2.0 最终审查修复 | 7 个 Important 问题修复：系统提示词、配置文件、Nudge i18n、死循环、死代码、68/68 测试通过 |

| **版本** | 0.2.0 |
| **LLM** | DeepSeek / Ollama / OpenAI / Anthropic (2个占位) |
| **架构** | 5-crate workspace: qbird-code-models / infra / tools / agents / qbird-code |
| **二进制** | `qingbird` (crates/qbird-code) |

### 4 门禁

- `cargo build` — 编译通过
- `cargo clippy --all-targets -- -D warnings` — 零告警
- `cargo fmt --check` — 格式一致
- `cargo test` — 全通过

### 文件结构

- `crates/qbird-code-models/` — 核心类型 (ProviderKind/LlmMessage/LlmResponse/ToolDef)
- `crates/qbird-code-infra/` — 4 Provider + config + event + env
- `crates/qbird-code-tools/` — 工具定义与注册
- `crates/qbird-code-agents/` — ReAct 循环 (Concierge) + Subagent + 死循环检测 + Nudge 机制
- `crates/qbird-code/` — 二进制入口，CLI/TUI/路由
- `locales/` — 中英双语

## 收工仪式

修改后更新上面「当前」状态表 + 把旧日志行移到 WORKLOG.md。
