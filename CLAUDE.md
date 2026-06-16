# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.2 计划已就位**：文档生成于 2026-06-17，按用户决定分三阶段 D1-D4（债务清理）/ E1-E6（并行派发）/ F1-F6（TUI）循序推进。下一步：从 main 开 `v1.2` 分支并开始 Task D1（抽 cache_key_for_step helper） |
| **上次完成** | v1.1 收尾仪式 + PR #10 闭环：本地 main 快进同步到 b1ed212（origin/main，PR #10 merge commit，8efcaa3 → b1ed212 via 35dad77）→ 删本地 `chore/v1.1-ceremony-docs` 分支（35dad77）→ 远程分支已被 GitHub 自动清理（prune 确认）→ 仓库仅剩 main 一支 → cargo build 0 错 + clippy --all-targets 0 警告 + fmt --check 0 diff。**注意**：sansan1983 用普通 merge（2 个父提交）合并 PR #10，**绕过了仓库的 `required_linear_history` 保护**——这是仓库主端选择，与我们推送方无关 |
| **下次动作** | 用户下达「开始 v1.2」指令后：建 `v1.2` 分支（从 origin/main b1ed212 切）→ Task D1 cache_key_for_step helper 起步。按 3→2→1 用户优先级：D1-D4 → E1-E6 → F1-F6 |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-17 | v1.2 实施计划生成 | 写 `docs/superpowers/plans/2026-06-17-eflow-v1.2-implementation-plan.md`（2638 行，19 tasks：D1-D4 P1 债务清理 / E1-E6 step_to_layer 并行派发 / F1-F6 TUI 交互层 ratatui+crossterm）。用户按 3→2→1 排优先级。同会话更新 CLAUDE.md「当前状态」+「关键文件」+「架构图」+「当前版本」指向 v1.2 计划 |
| 2026-06-16 | v1.1 PR #10 文档同步闭环 | 建 `chore/v1.1-ceremony-docs`（从 origin/main 8efcaa3 切）→ 提交 35dad77（CLAUDE.md +3/-3，WORKLOG.md +2/-0）→ 推 → 开 PR #10（"chore: 同步文档状态至 PR #9 已合并"）→ sansan1983 普通 merge（非 squash，2 个父提交 8efcaa3 + 35dad77，**绕过 required_linear_history 保护**——仓库主端选择）@ 16:42 UTC → b1ed212 → 本地 main 快进同步（git pull --ff-only）→ 删本地 + 远程补丁分支 → 仓库仅剩 main → 完工门禁全 0 错 0 警告 |
| 2026-06-16 | v1.1 收尾仪式 + PR #9 闭环 | 用户提出"主分支保护下能直推吗"问题 → 核实分支保护配置（required_pull_request_reviews + required_linear_history + 禁 force push）→ 决定走补丁分支 + PR 流程 → 建 `chore/v1.1-ceremony`（从 origin/main 540e5cc 切）→ cherry-pick 773f970 → 55374d4 → 门禁 build/clippy/fmt 全 0 → 推补丁分支 → 开 PR #9（标题"chore: v1.1 收尾仪式 — 同步文档状态"）→ sansan1983 squash-merge @ 13:53 UTC → 8efcaa3 → 本地 main reset 到 origin/main → 删本地 + 远程补丁分支（远程已自动清）→ 仓库仅剩 main → 完工门禁全 0 错 0 警告 |

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
| `docs/superpowers/plans/2026-06-17-eflow-v1.2-implementation-plan.md` | **v1.2 实现计划（待执行，D1-D4 + E1-E6 + F1-F6）** |
| `docs/superpowers/plans/2026-06-15-eflow-v1.1-implementation-plan.md` | v1.1 实现计划（已收尾，归档：M4.5 + M8 + M10.5 + 跨阶段 D1-D4） |
| `docs/superpowers/plans/2026-06-15-eflow-v1.0-implementation-plan.md` | v1.0 实现计划（已收尾，归档） |
| `WORKLOG.md` | 完整工作日志归档 |

### 架构四层

```
交互层       →  TUI (ratatui, v1.2 计划 F1-F6) + CLI (--execute 单次模式)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+调度，并行派发 v1.2 计划 E1-E6)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  LLM / Memory / Context / Event / Profile / Tools
```

### 当前版本

v1.1.0 已发布（PR #8 merged @ 540e5cc）：M4.5 LLM 硬化 + M8 L2 结构化缓存 + M10.5 多 Subagent 并发池 + 跨阶段（base URL env var + --execute 事件 + L2 cache wiring）。**v1.2 计划已就位**（D1-D4 债务清理 / E1-E6 并行派发 / F1-F6 TUI 交互层，详见 v1.2 实现计划），等待用户「开始 v1.2」指令启动。

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml + lru（v1.1+ L2 缓存）+ ratatui + crossterm（v1.2 计划 F1-F6 TUI）
