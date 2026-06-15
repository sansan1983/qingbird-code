# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **等 PR #7 合入 + 重开 v1.1**（v1.1 计划已就绪待执行）|
| **上次完成** | v1.0.3 hotfix 完成 + 工程质检全过 — 154/154 pass / 0 clippy 警告 / fmt 干净 / PR #7 已开 |
| **下次动作** | 等 PR #7 (v1.0.3 → main) 合并 → 切回 v1.1 分支 → 开始 Phase A Task A1 (扩 LlmConfig) |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-16 | v1.0.3 hotfix + 工程质检 | v1.0.3 分支 (6 commits): 修真 2 bug (UTF-8 panic + task_id 'unknown') + 修 cli dead field + 8 处代码去重 (R1-R8) + 4 处 magic→const (M1+M2+M3+M5) + clippy pedantic 166 个警告 auto-fix。154/154 tests pass, 0 clippy 警告。PR #7 已开 |
| 2026-06-16 | v1.1 启动 + 暂停 | v1.1 分支已建（无 commit），基线测试发现 cli_test 2 fail → 确认为 v1.0 遗留 bug → 暂停 v1.1 转 v1.0.3 热补 |
| 2026-06-15 | v1.1 计划生成 | `docs/superpowers/plans/2026-06-15-eflow-v1.1-implementation-plan.md`（2985 行，覆盖 M4.5+M8+M10.5） |

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
| `docs/superpowers/plans/2026-06-15-eflow-v1.1-implementation-plan.md` | **v1.1 实现计划（按 Task + Step 执行，活跃中）** |
| `docs/superpowers/plans/2026-06-15-eflow-v1.0-implementation-plan.md` | v1.0 实现计划（已收尾，归档） |
| `WORKLOG.md` | 完整工作日志归档 |

### 架构四层

```
交互层       →  交互层/CLI (v1.2 → TUI)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+调度)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  LLM / Memory / Context / Event / Profile / Tools
```

### 当前版本

v1.1 计划已生成待执行：M4.5 LLM 硬化（关 QA B2）+ M8 L2 结构化缓存 + M10.5 多 Subagent 并发池

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml
