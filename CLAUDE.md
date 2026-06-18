# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.3.1 实施完成**：12/12 tasks 全部 commit（v1.3.1 branch 13 commits ahead of origin/v1.3.0）。**待用户决定下一步**：(a) **开 PR 把 v1.3.1 合回 main**（或合到 v1.3.0 branch 后一并推 main）；(b) **继续 v1.3.2 spec B2 实施**（CLI 契约，12 tasks）；(c) **继续 v1.3.3 spec C 实施**（3 档工作流，9 tasks）；(d) **先做 v1.4 spec D 头脑风暴**（渲染引擎 + TUI 重构） |
| **上次完成** | v1.3.1 spec B1 向导+斜杠命令实施收官。M1 SlashCommand / WizardStep / SelectItemSource trait + CommandRegistry 3 核心抽象 / M2 6 个 builtin 斜杠命令（`/model /profile /lang /level /help /quit`）/ M3 7 个 builtin 向导 step（welcome/language/provider/protocol/apikey/model/confirm）+ Wizard 状态机 / M4 TUI 焦点感知 + Concierge 斜杠分发 + main.rs `eflow init` + 首次启动检测 / M5 21 个新 i18n key + CHANGELOG + Cargo.toml bump 1.2.0→1.3.1（顺带覆盖 v1.3.0 漏的 bump） / M6 4 门禁 + T24 TODO 锚点 + 14 步手工验证清单。**283 tests pass**（v1.3.0 234 + v1.3.1 +49）。**6 处 plan 偏差**已文档化：#11a CommandContext router `Arc<Mutex<>>` / #11b Concierge::new 加 router 参数 / #11c concierge.rs 4 处硬编码 → T11 已走 i18n key 修正 / #11d `with_bare_mode` T10 暂未在 main 显式调 / #11e Cargo.toml 1.2.0→1.3.1 一步 bump（覆盖 v1.3.0 漏的状态） / #11f 14 步手工验证需真 TTY 写文档请 reviewer 跑 / #11g SelectList TODO 留 v1.3.2 spec B2 统一处理 |
| **下次动作** | 等用户决定：(a) **开 PR v1.3.1 → main**（13 commits：M1-M6 + 起点空 commit + v1.3.0 merge commit）；(b) **继续 v1.3.2 spec B2 实施**（CLI 契约 12 tasks）；(c) **继续 v1.3.3 spec C 实施**（3 档工作流 9 tasks）；(d) **先做 v1.4 spec D 头脑风暴**（渲染引擎 + TUI 重构） |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-17 | v1.3.1 实施收官 | 12/12 tasks 全 commit（v1.3.1 branch 13 commits ahead of origin/v1.3.0）：M1 trait 抽象 / M2 6 builtin slash / M3 7 builtin wizard + 状态机 / M4 TUI 焦点感知 + Concierge 分发 + main.rs init / M5 21 i18n key + CHANGELOG + Cargo.toml 1.2.0→1.3.1 / M6 4 门禁 + T24 TODO + 14 步手工验证清单。**283 tests pass**（v1.3.0 234 + v1.3.1 +49）。**6 处 plan 偏差**全文档化（#11a-g）。**破坏性变更**：无（v1.3.0 已有破坏：`EflowConfig::llm.providers` 删除） |
| 2026-06-17 | v1.3.0 实施收官 | 26/26 tasks 全 commit（v1.3.0 branch 18 commits ahead of origin/main）：M1 公共 + `ProviderConfig` / M2 `PresetLoader` / M3 `LlmProviderRegistry` + 2 Generic adapter / M4 `LlmRouter::from_config` 重写（**破坏性**：`EflowConfig::llm.providers` 字段删除） / M5 main.rs + i18n + CHANGELOG + 迁移文档 / M6 4 门禁 + 233 tests pass / M7 稳定性约束（trait 冻结 + glob 测试 + ADR 引用）。**2 处 plan 偏差**：#6 `anthropic.rs`/`openai.rs` 文件未删（deviation deferred）；#7 T23 跳过（forward-compat 是 serde_yaml 默认行为，TDD Iron Law 不可违反） |
| 2026-06-17 | v1.3 writing-plans 收官 | 4 个 plan：spec A（5a7dfef 3135 行 / 26 tasks）/ B1（54a5515 3612 行 / 12 tasks）/ B2（4fba0a9 2141 行 / 12 tasks）/ C（31713b2 1478 行 / 9 tasks）。总 59 tasks 分 4 个小版本发布 v1.3.0/1.3.1/1.3.2/1.3.3。每个 plan 3 项自审（spec coverage / placeholder / type consistency）全过 |

## △ 收工仪式（每次结束前执行）

会话结束时，更新上面「当前状态」表格：
1. 把「上次完成」改为刚才做了什么
2. 把「下次动作」改为下一个 Task 是什么
3. 把新的日志行追加到「近期日志」顶部，保留最近 3 条
4. 把旧日志行移到 `WORKLOG.md` 尾部

---

<!--
     ╔════════════════════════════════════════╗
     ║  §2 不可变铁律 — 每次编码前必须遵守  ║
     ╚════════════════════════════════════════╝
-->

## 铁律

### 1. Think Before Coding

- 先陈述假设。不确定就问。
- 有多种解释时，列出来让用户选——不要自己猜。
- 有更简单的方案就说。该 push back 就 push back。
- 遇到不清楚的地方，停下来，指出困惑点。

### 2. Simplicity First

- 不做需求以外的功能。
- 不为单次使用写抽象。
- 不做没被要求的"灵活性"或"可配置性"。
- 不对不可能出现的场景写错误处理。
- 写了 200 行能压到 50 行，就重写。

### 3. Surgical Changes

- 不改相邻代码的注释、格式、风格。
- 不重构没坏的东西。
- 匹配现有风格，即便不是你的偏好。
- 发现无关的 dead code，提出来——但别删。
- 只清理你引入的 orphan（import、变量、函数）。

### 4. Goal-Driven Execution

- 每个任务写成可验证目标："加验证" → "先写失败测试，再让它通过"。
- 多步骤任务先列简要 plan，每步带 verify 检查点。
- 不写模糊的成功标准。

### 5. 开工必做

- **读 CLAUDE.md 顶部「当前状态」表格** — 这就是你的接班指令
- **读实现计划文件** — 确认当前 Task 的详细步骤
- **开工前说一句**你在做什么 — "开始 Task X: [描述]"
- **严格按 Task 的 Step 顺序执行** — 每个 Step 做完再进下一个

### 6. 交接必做

- **更新 CLAUDE.md 顶部「当前状态」表格**
- **把旧日志移到 WORKLOG.md**
- commit 时写清楚做了什么

### 7. Rust 特定

- `cargo build` 通过才算编译成功
- `cargo clippy` 零告警（warning = build fail）
- `cargo fmt` 通过
- `cargo test` 全部通过
- async 代码用 tokio，不混用其他 runtime

---

<!--
     ╔══════════════════════════════════════════╗
     ║  §3 项目速览 — 理解项目结构用           ║
     ╚══════════════════════════════════════════╝
-->

## 项目速览

**eflow**: Rust 多层 Agent 协作框架。零阻塞对话 + 三角色分层决策。

### 关键文件

| 文件 | 用途 |
|------|------|
| `docs/superpowers/specs/2026-06-15-eflow-design.md` | 架构设计文档 v4.0（理解架构读这个） |
| `docs/superpowers/specs/2026-06-17-eflow-v1.3-llm-abstract-design.md` | v1.3 spec A — LLM 抽象扩展（21-27 tasks） |
| `docs/superpowers/specs/2026-06-17-eflow-v1.3-b1-wizard-slash-design.md` | v1.3 spec B1 — 向导 + 斜杠命令（12 tasks） |
| `docs/superpowers/specs/2026-06-17-eflow-v1.3-b2-cli-contract-design.md` | v1.3 spec B2 — CLI 契约（13 tasks） |
| `docs/superpowers/specs/2026-06-17-eflow-v1.3-c-workflow-levels-design.md` | v1.3 spec C — 3 档工作流（7-8 tasks） |
| `docs/superpowers/plans/2026-06-17-eflow-v1.3.0-llm-abstract-plan.md` | v1.3.0 实施计划（spec A 实施，26 tasks / 7 milestones） |
| `docs/superpowers/plans/2026-06-17-eflow-v1.3.1-wizard-slash-plan.md` | v1.3.1 实施计划（spec B1 实施，12 tasks / 6 milestones） |
| `docs/superpowers/plans/2026-06-17-eflow-v1.3.2-cli-contract-plan.md` | v1.3.2 实施计划（spec B2 实施，12 tasks / 8 milestones） |
| `docs/superpowers/plans/2026-06-17-eflow-v1.3.3-workflow-levels-plan.md` | v1.3.3 实施计划（spec C 实施，9 tasks / 5 milestones） |
| `docs/superpowers/plans/2026-06-17-eflow-v1.2-implementation-plan.md` | v1.2 实现计划（已收尾：D1-D4 + E1-E6 + F1-F6 全完成） |
| `docs/superpowers/plans/2026-06-15-eflow-v1.1-implementation-plan.md` | v1.1 实现计划（已收尾，归档：M4.5 + M8 + M10.5 + 跨阶段 D1-D4） |
| `docs/superpowers/plans/2026-06-15-eflow-v1.0-implementation-plan.md` | v1.0 实现计划（已收尾，归档） |
| `WORKLOG.md` | 完整工作日志归档 |

### 架构四层

```
交互层       →  TUI (ratatui) + CLI (--execute 单次模式)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+调度，按层并行)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  LLM / Memory / Context / Event / Profile / Tools
```

### 当前版本

v1.2.0 已发布（v1.2 branch 17 commits + origin/v1.2 推送，待开 PR）：D1-D4 P1 债务清理（cache_key helper / Concierge 真切换 / recall memory）+ E1-E6 并行派发（SubagentHandle guard / list_active / compute_step_layers / FuturesUnordered 按层执行 / cleanup_idle timeout-based）+ F1-F6 TUI 交互层（ratatui+crossterm / InteractionLayer trait / TuiBackend event loop + state machine / main.rs 默认 TUI 启动）。

**v1.3 设计完成待 plan**：A（LLM 抽象扩展）/ B1（向导+斜杠命令）/ B2（CLI 契约）/ C（3 档工作流）共 4 个 spec，53-60 tasks，分 3 个小版本 v1.3.0 / v1.3.1 / v1.3.2。spec D（渲染引擎重构，留给 v1.4）作为 v1.3 已知偏差标注。

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml + lru（L2 缓存）+ ratatui + crossterm（TUI）

**v1.3 新增技术决策**（spec 阶段确认）：
- LLM 抽象：核心零预置，provider 元数据在 `~/.eflow/providers/*.yaml`
- 交互层：`SlashCommand` trait + `CommandRegistry`（6 个 builtin 命令）+ `WizardStep` trait（7 个 builtin step）+ `SelectItemSource` trait + `SelectList` widget（多模交互）
- 工作流：`WorkflowExecutor` trait + `WorkflowRegistry`（3 个 builtin 档位 Simple/Standard/Advanced）
- CLI 契约：`eflow session start` 持续运行 + stdin 协议 + 契约冻结 v1.3.0 起
- v1.4 候选：spec D 渲染引擎（`RenderEngine` trait + `DrawCommand` enum）
