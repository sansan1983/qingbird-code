# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.3.3 spec C 全部收官 + 待 push**。v1.3.3 branch 上 8 commits / 4 门禁全过（335 tests / 0 clippy / 0 fmt）。**v1.3 阶段总览**：v1.3.0 spec A（LLM 抽象）+ v1.3.1 spec B1（Wizard + SlashCommand）+ v1.3.2 spec B2（CLI 契约）+ v1.3.3 spec C（3 档工作流）= **59 tasks** 全部完成。**v1.3 spec C 收官**。**下一阶段决策**：v1.4 spec D（渲染引擎 RenderEngine trait + DrawCommand enum，留给 v1.4 解决 v1.3.1 known 偏差）vs 维护模式 — 等用户确认 |
| **上次完成** | v1.3.3 spec C 实施 8 commits：0d75384（trait 抽象）+ 5fd84a1（fmt）+ 3a26a65（3 个 builtin 档位）+ 702a095（档位测试 7 个）+ dc66609（Concierge 加 workflow_registry + 5 规则 + dispatch_task_with_level）+ 8b0d2ad（placeholder + 8 个规则测试）+ ae92699（/level 命令覆盖空壳）+ 3da211d（main.rs 注册 + 8 locale key + CHANGELOG + Cargo 1.3.3）。4 门禁：build ✓ / 335 tests ✓ / 0 clippy / 0 fmt。**deviation #13a-l**（11 个 plan 偏差）记录在 commit messages |
| **下次动作** | 等用户确认 push v1.3.3 + 开 PR #17（v1.3.2 收工 commit 6501365 仍本地未 push）。**下一阶段决策**： |
  1. **v1.4 spec D** —— 渲染引擎重构（RenderEngine trait + DrawCommand enum，留给 v1.4 解决 v1.3.1 known 偏差：WizardStep/SelectList/TuiBackend 直接调 ratatui API 违反"零硬编码"）
  2. **暂不开发** — 维护模式 |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-18 | v1.3.3 spec C 收官 | v1.3.3 branch 上 8 commits：0d75384 trait 抽象 + 5fd84a1 fmt + 3a26a65 3 builtin 档位 + 702a095 档位测试 7 个 + dc66609 Concierge 5 规则 + 8b0d2ad placeholder + 8 规则测试 + ae92699 /level 覆盖空壳 + 3da211d main.rs + i18n + CHANGELOG + Cargo 1.3.3。4 门禁全过（335 tests / 0 clippy / 0 fmt）。**11 个 deviation #13a-l**：#13a Concierge Arc<Mutex<>> + #13b TaskSpec 缺 workflow_level + #13c AggregatedResult 新建 + #13d Concierge placeholder + #13e llm_router_handle + #13f dispatch_standard/with_retries 缺 + #13g blackboard_mut 缺 + #13h Advanced 1 次反馈不真做 3 次 + #13i description hard-code + #13j CompositeMemory recall_smart 不用 trait + #13k keyword case-insensitive + #13l 14 步 TUI 验证 sandbox skip。**v1.3 59 tasks 全部完成** |
| 2026-06-18 | v1.3.2 PR #16 merged | `gh pr view 16 --json state` = MERGED, mergedAt 2026-06-18T06:29:43Z, merge commit aa73ddb。`git checkout main && git pull --ff` 拿到 11 commits / 24 文件 / +1445 -51。清理：`git branch -d v1.3.2` + `git remote prune origin` 删 4 个 stale refs (v1.2 / v1.3.0 / v1.3.1 / v1.3.2)。main 上 4 门禁 + Python 8/8 全绿。**v1.3 spec B2 实施收官** |
| 2026-06-18 | v1.3.2 PR #16 创建 | `git push -u origin v1.3.2` 成功；`gh pr create --base main --head v1.3.2` 10 commits ahead of origin/main。PR body 含 12 tasks 摘要 / 4 门禁 / 22 deviations 摘要 / 3 步 manual review / docs/cli-contract.md + tests/gui_smoke_test.py 链接。**#16 OPEN 等 reviewer 合并** |
|------|------|------|
| 2026-06-18 | v1.3.2 M8 commit 5f4b5e1 | 4 门禁全过（build / 311 tests / 0 clippy / 0 fmt）；10 步手工验证（Step 3-6 + 9-11 跑通，Step 2 + 7-8 manual skip 需 reviewer 跑）；CHANGELOG v1.3.2 段（features 2 subcommand + 7 事件 + 5 stdin + 4 exit / internal 3 ADR + 5 deviations / upgrade notes）。**v1.3.2 完整收官 9 commits** |
| 2026-06-18 | v1.3.2 M7 commit | docs/cli-contract.md 完整契约（7 事件 / 5 stdin / 4 exit / Python 示例 + 3 deviations）；tests/fixtures/{mock_config, providers/mock, profiles/test}.yaml；tests/gui_smoke_test.py 8 步全过。**#12v critical**：tracing 走 stdout 破坏 GUI 契约（stdout 必 JSON）—— main.rs 加 .with_writer(stderr) 修。**#12w** mock config 按 v1.3.1 真实 schema 写（不抄 plan）。**#12x** smoke test 不调 send（mock 不可达）。**#12y** run_eflow_session 默认加 --config flag。**Python 8/8 pass** |
| 2026-06-18 | v1.3.2 M6 commit ff82026 | Event::SystemReady { task_id, started_at: SystemTime } variant + 2 测试。tui.rs:293 match 加 _ => 兜底（SystemReady 是 CLI-only，TUI 不消费，spec ADR-0016 TUI 零改造）；start.rs:193 match 加 SystemReady 分支（**当前未流通**——start.rs 仍 M3 手写 NDJSON，保留分支供未来复用）。**#12n**：plan 假设 start.rs 改用 publish(SystemReady)+listener 转发 —— 实际 M3 已手写 JSON 写到 stdout，本次只加 variant，**不**改 start.rs 路径。**test 311 pass**（309 + 2 event） |


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
