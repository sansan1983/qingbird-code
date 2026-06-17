# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **v1.2 收尾完成**：17 commits ahead of origin/main（v1.2 branch 已推 `origin/v1.2`，未开 PR）。17 commits 拆为：D1-D4 + Phase D 收尾（4）+ E1-E6 + Phase E 收尾（7）+ F1-F6 + Phase F 收尾（5）+ 跨阶段 v1.2.0 版本号 bump（1）。4 门禁全过：build / clippy -D warnings / fmt --check / cargo test。下一步：v1.3 候选（向量记忆 / GUI 扩展 / OpenAI streaming / 5 步骤独立 plan 并行加速 e2e 测试） |
| **上次完成** | v1.2 全部 19 tasks（D1-D4 / E1-E6 / F1-F6）落地 + 跨阶段收尾：版本号 1.1.0 → 1.2.0、CHANGELOG Unreleased 段加「TUI 交互」「并行派发」「P1 债务清理」、README 状态表加 v1.2 行 + 架构图标注「TUI (ratatui, v1.2) + CLI (--execute)」+ 编排层加「v1.2 按层并行」。v1.2 branch 17 commits + 0 错 0 警告 0 fmt diff 全绿 |
| **下次动作** | 等用户决定：(a) **开 PR 把 v1.2 合回 main**（分支已推送，PR 创建是仓库主端选择）；(b) **直接 merge main 后 delete v1.2 branch**；(c) **开始 v1.3**（候选：向量记忆 + L3 语义缓存 / GUI egui-iced / OpenAI chat_stream / 5 步骤独立 plan 并行加速 e2e 集成测试） |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-17 | v1.2 全部 19 tasks 落地 | D1-D4 抽 cache_key_for_step helper / Concierge 真切换 active_profile / Concierge recall memory before dispatch；E1-E6 SubagentHandle guard / list_active / compute_step_layers / FuturesUnordered 按层并行 / cleanup_idle timeout-based / parallel_execution_test；F1-F6 ratatui+crossterm / InteractionLayer trait / TuiBackend event loop + state machine + prompt input / main.rs 默认 TUI。17 commits ahead of origin/main，4 门禁全绿，v1.2 已推 origin/v1.2 未开 PR |
| 2026-06-17 | v1.2 实施计划生成 | 写 `docs/superpowers/plans/2026-06-17-eflow-v1.2-implementation-plan.md`（2638 行，19 tasks：D1-D4 P1 债务清理 / E1-E6 step_to_layer 并行派发 / F1-F6 TUI 交互层 ratatui+crossterm）。用户按 3→2→1 排优先级。同会话更新 CLAUDE.md「当前状态」+「关键文件」+「架构图」+「当前版本」指向 v1.2 计划 |
| 2026-06-16 | v1.1 PR #10 文档同步闭环 | 建 `chore/v1.1-ceremony-docs`（从 origin/main 8efcaa3 切）→ 提交 35dad77（CLAUDE.md +3/-3，WORKLOG.md +2/-0）→ 推 → 开 PR #10（"chore: 同步文档状态至 PR #9 已合并"）→ sansan1983 普通 merge（非 squash，2 个父提交 8efcaa3 + 35dad77，**绕过 required_linear_history 保护**——仓库主端选择）@ 16:42 UTC → b1ed212 → 本地 main 快进同步（git pull --ff-only）→ 删本地 + 远程补丁分支 → 仓库仅剩 main → 完工门禁全 0 错 0 警告 |

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

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml + lru（L2 缓存）+ ratatui + crossterm（TUI）
