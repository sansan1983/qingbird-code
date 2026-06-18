# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.3.2 spec B2 实施中（7/12 tasks done = M1-M7）**：M1+M2 CliOutput + exit_code / M3 session start / M4 stdin 协议 + 5 handlers / M5 init Wizard / M6 Event::SystemReady / M7 契约文档 + mock fixture + Python 集成测试（8 步全过）。**下一动作**：M8 T12 4 门禁 + 10 步手工验证清单 + CHANGELOG.md v1.3.2 段 |
| **上次完成** | v1.3.2 M1-M7（commit M7）— 8 commits on v1.3.2 branch。**M7 关键产出**：docs/cli-contract.md + tests/fixtures/* + gui_smoke_test.py 8/8 pass。**#12v critical**：tracing 走 stdout 破坏 GUI 契约（stdout 必 JSON），main.rs 加 `.with_writer(stderr)` 修。**累计 deviations #12a-v（22 个）**。**测试**：311 pass + Python 8/8 pass。**门禁**：build ✓ / 311 ✓ / 0 clippy / 0 fmt |

| **下次动作** | 开工 v1.3.2 M8 T12：（1）4 门禁（build/test/clippy/fmt）；（2）10 步手工验证清单（plan T12 Step 2-11）；（3）CHANGELOG.md v1.3.2 段（features / internal / upgrade notes）。**手工验证需真跑 eflow 二进制**——可能需 mock provider install + 验证 stderr 行为 |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 2026-06-18 | v1.3.2 M7 commit | docs/cli-contract.md 完整契约（7 事件 / 5 stdin / 4 exit / Python 示例 + 3 deviations）；tests/fixtures/{mock_config, providers/mock, profiles/test}.yaml；tests/gui_smoke_test.py 8 步全过。**#12v critical**：tracing 走 stdout 破坏 GUI 契约（stdout 必 JSON）—— main.rs 加 .with_writer(stderr) 修。**#12w** mock config 按 v1.3.1 真实 schema 写（不抄 plan）。**#12x** smoke test 不调 send（mock 不可达）。**#12y** run_eflow_session 默认加 --config flag。**Python 8/8 pass** |
| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-18 | v1.3.2 M6 commit ff82026 | Event::SystemReady { task_id, started_at: SystemTime } variant + 2 测试。tui.rs:293 match 加 _ => 兜底（SystemReady 是 CLI-only，TUI 不消费，spec ADR-0016 TUI 零改造）；start.rs:193 match 加 SystemReady 分支（**当前未流通**——start.rs 仍 M3 手写 NDJSON，保留分支供未来复用）。**#12n**：plan 假设 start.rs 改用 publish(SystemReady)+listener 转发 —— 实际 M3 已手写 JSON 写到 stdout，本次只加 variant，**不**改 start.rs 路径。**test 311 pass**（309 + 2 event） |
| 2026-06-18 | v1.3.2 M5 commit 327a85e | cli/init.rs 搬 v1.3.1 main.rs::run_init_wizard 改 i32 exit code（0/1/2）；main.rs 删 30 行 wizard 代码 + init 路由调 cli::init::run() + 首次启动 fallback 也走 cli::init。**#12m**：plan 假设 init 需要 LlmRouter + EflowConfig::default() —— 实际 Wizard 走 stdin/stdout，**不**需要 router，零 router 代码。**test 309 pass**（持平） |
| 2026-06-18 | v1.3.2 M4 commit 305b2a5 | StdinCommand 5 变体 enum + 7 serde round-trip 测试 + read_loop 持续读 stdin + 5 handlers 真 dispatch。**handler 策略**：send→handle_input / end→stderr+返 0 / level→dispatch_slash / lang→locale::init / help→registry.list()。**#12l**：Concierge::dispatch_slash 改 pub（1 行 surgical）。**test 309 pass**（302 + 7 stdin） |
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
