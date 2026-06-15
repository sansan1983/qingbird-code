# CLAUDE.md — eflow 项目

<!--
     ╔══════════════════════════════════════════════════════╗
     ║  △ 会话交接区 — 每次开工先读这里，每次收工更新这里  ║
     ╚══════════════════════════════════════════════════════╝
-->

## △ 当前状态

| 项目 | 内容 |
|------|------|
| **当前任务** | M7 已完成，等待开工 M8 |
| **上次完成** | M7 Profile + Skill 系统（ProfileRegistry YAML 加载 + SHA256 checksum + Skill 模板合并，15 测试通过） |
| **下次动作** | 开工 M8 工具系统（Tool trait + Registry + read_file/write_file/exec/search） |

**近期日志**（最近 3 条，完整历史见 `WORKLOG.md`）：

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-15 | M7 Profile + Skill | `src/infrastructure/profile/{mod,loader,skill}.rs` + `profiles/developer.yaml` + `tests/profile_test.rs` |
| 2026-06-15 | M6 上下文管理 | `src/infrastructure/context/{mod,reference,compressor}.rs` + `tests/context_test.rs` |
| 2026-06-15 | M5 记忆系统（三层） | `src/infrastructure/memory/{mod,manager,working,project,user,composite}.rs` + `tests/memory_test.rs` |

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
| `docs/superpowers/plans/2026-06-15-eflow-v1.0-implementation-plan.md` | v1.0 实现计划（按 Task + Step 执行） |
| `WORKLOG.md` | 完整工作日志归档 |

### 架构四层

```
交互层       →  交互层/CLI (v1.2 → TUI)
编排层       →  Concierge (零阻塞) → Orchestrator (分解+调度)
能力层       →  Decisioner → Executor → Feedbacker (管线段)
基础设施层   →  LLM / Memory / Context / Event / Profile / Tools
```

### 当前版本

v1.0 内核范围：端到端可运行骨架（单 Subagent、三层记忆、CLI 交互）

### 技术栈

Rust 2024 + tokio + clap + reqwest + rusqlite + serde_yaml
