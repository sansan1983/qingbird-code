# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **代码体检 PR #26 merged**：audit 10,245 行代码 → 发现 14 项（P0×2 / P1×4 / P2×4 / P3×4）→ 实施 #1-#6（P0+P1 共 6 项低难度修复，5 files +26/-8）→ PR #26 merged → milestone/v1.4 = 最新。**4 门禁状态**：build ✓ / test ✓ (155 passed) / clippy ✓ / fmt ✓（本地未装 rustfmt 组件）|
| **上次完成** | PR #26 merged (code audit #1-#6 fixes) → `git push origin chore/code-audit-fixes-pr` → `gh pr create --base milestone/v1.4` → PR #26 https://github.com/sansan1983/eflow/pull/26 merged。**修复 6 项**：registry unwrap → map_err / token field fallback / choices bounds / mutex annotation / error body / cache lock fallback |
| **下次动作** | **等用户决定下一步**：(1) README 拆分 PR / (2) P2 优化（router routing 合并、env var fallback 去重）/ (3) v1.4 spec D 实施 PR1（远程服务器）。**本地分支**：milestone/v1.4 (up to date with origin/milestone/v1.4) |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-18 | PR #22 收工仪式 | PR #22 user squash-merged → milestone/v1.4 mergeCommit `d15a4ad` (2 commits / 7 files / +44 -25)：a0a0c1a (cherry-pick 43f3728 PR A followup：删 `concierge.llm_router_handle()` 死方法 + main.rs 启动失败路径 3 处 eprintln 改 `t!()` + 3 个 locale key) + c781702 (README 改 v1.3.3 状态 166 行 + AGENTS.md 4 层图删 workflow 引用 + 删 `local-llm` feature flag)。`git checkout milestone/v1.4 && git pull --ff` 拿到 d15a4ad。清理：删本地 3 分支（chore/docs-and-misc-pr-b / chore/dead-code-cleanup-pr-a / chore/dead-code-cleanup-pr-a-followup + 中途分支 chore/misc-cleanup-and-docs-pr-b）+ `git push origin --delete chore/dead-code-cleanup-pr-a` 删远程 stale ref + `git remote prune origin` 删 3 stale ref。local: milestone/v1.4 = d15a4ad；remote: origin/main + origin/milestone/v1.4 双分支。**4 门禁全过**：build ✓ / test ✓ (312 passed) / clippy ✓ / fmt ✓。**代码体检 4 PR 全部完成**：PR #19 ✅ / #20 ✅ / #21 ✅ / #22 ✅。**下一轮 PR**：README 拆分成中/英两份（用户偏好）。**v1.4 spec D 实施 PR1** 仍待远程服务器 |
| 2026-06-18 | 代码体检 PR #21 死代码清理 merged | PR #21 user squash-merged @ 2026-06-18T12:36:30Z, mergeCommit 1be88b8 (20 files / +76 -1380)：db7b42a (删 v1.3.3 workflow 抽象整套 513 行 + 死 LLM client 538 行 + 4 死字段 + 5 死方法 + 8 dead locale key + 死函数)。**/level 命令从"假切档"改"占位提示"**——承认 v1.3.3 spec C 实施未接通。后续 43f3728 followup (llm_router_handle 死方法 + main.rs 3 处 eprintln 改 t!) cherry-pick 到 PR #22。**312 tests 全过**（之前 335 - 23 = 312）。**4 门禁**：build ✓ / clippy ✓ / test ✓ / fmt ✓ |
| 2026-06-18 | AGENTS.md 收尾仪式 + PR #19 merged | PR #19 user squash-merged @ 2026-06-18T11:32Z → milestone/v1.4 mergeCommit `ae2e3eb` (2 files / +96 -0)。`git checkout milestone/v1.4 && git pull --ff` 拿到 ae2e3eb。清理：`git branch -d feature/docs-agents-md`（was 4d6b560）+ `git remote prune origin` 删 stale ref。local: milestone/v1.4（up to date with origin/milestone/v1.4 ae2e3eb）；remote: origin/main + origin/milestone/v1.4 双分支。**4 门禁全过**：build ✓ / clippy ✓ / fmt ✓ / test ✓。**AGENTS.md 内容**：9 节（项目概览 / 4 门禁 / 分支 PR 流程 / Surgical / i18n 严格 / stdio 契约 / 约定 / 容易踩的坑 / 文件地图），149 行英文版 → 95 行中文版（用户"英文不熟悉" → 中文化，保留命令/路径/技术术语英文）。**v1.4 阶段**：milestone/v1.4 拉好 + PR #19 合并（v1.4 阶段首个 PR），v1.4 spec D 文档已合 main (PR #18)，**实施 PR1 (v1.4.0-abstract) 仍待远程服务器做** |

|------|------|------|


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
