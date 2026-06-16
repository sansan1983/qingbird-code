# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | **idle**：v1.1 收尾完毕，云端与本地 main 完全同步（均 540e5cc，PR #8 squash merged by sansan1983 @ 12:10 UTC），仓库仅剩 main 一支，cargo build/clippy/fmt 全 0 告警；等待用户下一步指示 |
| **上次完成** | v1.1 收尾清理：本地 main fast-forward 同步到 540e5cc（diff 0）→ 验证 v1.1 squash merge 内容已全在 main（version 1.1.0 + L2 cache + BASE_URL env var + --execute 事件等待）→ 删 v1.1 本地+远程（5 个未合入 commit 内容已被 squash 包含，diff v1.1..main = 0 无丢失）→ cargo build 0 错 0 警告 + clippy --all-targets 0 警告 + fmt --check 0 diff。**Plan bug 修 1 处**：用户以为 v1.1 与 main 是 full merge，实际是 squash merge + 5 个 post-merge commit，幸亏 diff 验证发现内容已含 → 安全 force-delete |
| **下次动作** | 等待用户下达下一步指示（v1.2 候选：step_to_layer 并行派发 + P1/P2 债务清理） |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-16 | v1.1 收尾清理 | PR #8 云端 MERGED (540e5cc squash merge by sansan1983 @ 12:10 UTC) → 本地 main 拉取同步 (diff 0) → 删 v1.1 本地+远程（5 post-merge commit 内容已被 squash 包含）→ 仓库仅剩 main 一支 → cargo build/clippy/fmt 0 错 0 告警 0 diff。**Plan deviation 1 处**：用户假设 v1.1 跟 main 是 full merge，实际是 squash merge，幸亏 diff 验证发现内容无丢失 |
| 2026-06-16 | v1.1 跨阶段 + D1-D4 收尾 | 5 commits: ac66d7c (bump v1.1.0 + CHANGELOG + README) + bc4ea90 (--execute 等事件) + 75e3f3c (base URL env var) + b3cc335 (base URL 语义修正) + e56a99e (L2 cache 接 capability 层)。e2e 用 minimaxi proxy 跑通：Run 1 6.6s → Run 2 0.2s (31× 加速)。修复 2 真 bug：base URL 缺 /v1/messages 拼接 + L2 cache 死代码。完工门禁 186/186 稳定 + 0 clippy 告警 + 0 fmt diff + 0 leftover。Push 待做 |
| 2026-06-16 | v1.1 Phase C 收尾 | 6 commits: 2e9b769 (Pool + mpsc) + 73e7da5 (Handle RAII) + ea5217e (role→cap 路由) + defbfce (Orchestrator pool + step_to_layer) + 0cc3178 (permission boundary + cleanup_idle) + 150c2a0 (pool 集成测试 + main 注入 pool)。M10.5 = 100%。Phase C 全关。Plan deviations 15 处（commit body 明文）。完工门禁 180/180 稳定 + 0 clippy 告警 + 0 fmt diff + 0 leftover。Push 待做 |

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
| `docs/superpowers/plans/2026-06-15-eflow-v1.1-implementation-plan.md` | v1.1 实现计划（已收尾，归档：M4.5 + M8 + M10.5 + 跨阶段 D1-D4） |
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

v1.1.0 已发布（PR #8 merged @ 540e5cc）：M4.5 LLM 硬化 + M8 L2 结构化缓存 + M10.5 多 Subagent 并发池 + 跨阶段（base URL env var + --execute 事件 + L2 cache wiring）。等待 v1.2 启动

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml
