# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.4 spec D 文档合并到 main + 等远程服务器实施 PR1**。local: main (up to date with origin/main 5a7563ea)。remote: origin/main 单分支。**PR #18 状态**：MERGED @ 2026-06-18T10:40:39Z, mergeCommit `5a7563ea`, 3 commits (6ffdfc7 spec / 6acf8b7 plan / 09be699 CLAUDE.md), +2382 -6, 3 files (2 docs + 1 CLAUDE.md)。**v1.4 spec D 范围**（电脑主机架构根治 v1.3.1 §12 已知偏差 + atlas 失败教训）：3 层架构（ViewModel → RenderEngine → RenderBackend）+ 5 个 ViewModel + 5 类 DrawCommand + 2 trait + 4 铁律 + 5 类测试 + ADR-0017 提案。**11 tasks 实施计划**：PR1 (v1.4.0-abstract, 抽象层 6 commits) + PR2 (v1.4.0-apply, 应用层 9-10 commits) + 收尾（ADR-0017 正式文件 3 commits）= 18-19 commits。**排除范围 7 项**：GUI backend / Theme 抽象 / Field 抽象 / i18n 接入 / 完整 DrawCommand 覆盖 / 鼠标精细控制 / 多 backend 一致性测试。**v1.3 阶段总览**（已完成）：v1.3.0 spec A (26) + v1.3.1 spec B1 (12) + v1.3.2 spec B2 (12) + v1.3.3 spec C (9) = 59 tasks。**⚠ 本地 4 门禁状态**：build ✓ (59.56s) / test ✓ (335 tests) / clippy ✓ (28.22s) / **fmt ✗ pre-existing**（rustfmt 组件未装，环境问题非 v1.4 PR 引入；远程服务器实施 PR1 时需先 `rustup component add rustfmt`） |
| **上次完成** | v1.4 spec D 收工仪式：PR #18 user merged @ 2026-06-18T10:40:39Z → `git checkout main && git pull --ff` 拿到 5a7563ea (3 files / +2382 -6) → 删本地分支 v1.4-rendering-pipeline (was 09be699) → `git remote prune origin` 删 stale ref → 跑 4 门禁（3/4 过 + 1 pre-existing fmt）→ 用户决定 fmt 留下次会话处理 → 待更新 CLAUDE.md + commit + push |
| **下次动作** | **在远程服务器上实施 v1.4 spec D PR1 (v1.4.0-abstract)**： |
  1. **远程服务器**：拉 main（拿到 v1.4 spec + plan 文档）→ `git checkout -b v1.4.0-abstract` → 读 plan 文档 tasks 1-4 + 11a → 实施（5 新文件 + 测试基础设施，6 commits）→ `rustup component add rustfmt` 装 fmt 组件 → 4 门禁全过 → push + `gh pr create --base main --head v1.4.0-abstract` 开 PR1
  2. **本地 review PR1** → 批准 / 反馈
  3. **merge PR1 → PR2 (v1.4.0-apply) 实施应用层重构**（tasks 5-10 + 11b，9-10 commits）
  4. **fmt pre-existing 待修**：下次本地会话重跑 `rustup component add rustfmt`（用户操作）+ 验证 4 门禁全过 |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-18 | v1.4 spec D 收工仪式 | PR #18 user merged @ 2026-06-18T10:40:39Z, mergeCommit 5a7563ea (3 commits / 3 files / +2382 -6)。`git checkout main && git pull --ff` 拿到 5a7563ea。清理：`git branch -d v1.4-rendering-pipeline` (was 09be699) + `git remote prune origin` 删 stale ref。local: main (up to date with origin/main 5a7563ea)。**本地 4 门禁**：build ✓ (59.56s) / test ✓ (335 tests / 1 doctest ignored) / clippy ✓ (28.22s) / **fmt ✗ pre-existing**（rustfmt 组件未装，环境问题非 v1.4 PR 引入；v1.3.3 收尾时 fmt 是过的，那时 rustfmt 装着；用户决定留下次会话处理）。**v1.4 spec D 范围已落地 main**：3 层架构（ViewModel → RenderEngine 显卡 → RenderBackend ratatui 驱动）+ 5 个 VM + 5 类 DrawCommand + 4 铁律 + 5 类测试 + ADR-0017 提案。**11 tasks 实施路径已就绪**：PR1 (v1.4.0-abstract) + PR2 (v1.4.0-apply) + 收尾 = 18-19 commits，**在远程服务器上实施**（本地只做设计/计划，不写代码） |
| 2026-06-18 | v1.4 spec D 文档交付 + PR #18 OPEN | brainstorming 全流程（澄清范围 → 4 关键决策 → 5 节设计 → spec 415 行 3 处补强 → plan 1960 行 4 处矛盾修正）→ `git checkout -b v1.4-rendering-pipeline` (from main 79bc55f4) → 2 commit（6ffdfc7 spec / 6acf8b7 plan, +2375 -0 2 files）→ `git push -u origin v1.4-rendering-pipeline` → `gh pr create --base main --head v1.4-rendering-pipeline` → PR #18 OPEN @ https://github.com/sansan1983/eflow/pull/18。**v1.4 spec D 范围**：3 层架构（ViewModel → RenderEngine 显卡 → RenderBackend ratatui 驱动）+ 5 个 VM + 5 类 DrawCommand + 4 铁律 + 5 类测试 + ADR-0017 提案。**11 tasks 拆分**：PR1 抽象层 6 commits + PR2 应用层 9-10 commits + 收尾 3 commits = 18-19 commits。**atlas 失败教训落地**（来自 EFO analysis 2026-06-14）：电脑主机架构焊死顶层 / 禁止任何"先 hardcode + TODO" 临时路径 / plan 30K 内 / 5 类测试齐全 / 单文件 < 200 行 + 字段 < 5 个。**排除范围 7 项**：GUI backend / Theme 抽象 / Field 抽象 / i18n 接入 wizard / 完整 DrawCommand 覆盖 / 鼠标精细控制 / 多 backend 一致性测试 |
| 2026-06-18 | v1.3.3 PR #17 merged + 收尾清理 | PR #17 MERGED @ 2026-06-18T07:59:09Z, mergeCommit 79bc55f4。`git checkout main && git pull --ff` FF 拿到 15 文件 / +888 -20。清理：`git branch -d v1.3.3`（b4d5577）+ `git remote prune origin` 删 1 个 stale ref (origin/v1.3.3)。local: main（up to date with origin/main 79bc55f4）；remote: origin/main 单分支。main 上 4 门禁全过：build ✓ (3.84s) / 334 tests ✓ (1 doctest ignored) / clippy --all-targets -- -D warnings 0 警告 (4.69s) / fmt --check 0 diff。**v1.3 阶段总览**：v1.3.0 spec A (26) + v1.3.1 spec B1 (12) + v1.3.2 spec B2 (12) + v1.3.3 spec C (9) = **59 tasks 全部完成**。下一阶段决策：v1.4 spec D 渲染引擎重构 vs 维护模式 — 等用户确认 |

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
