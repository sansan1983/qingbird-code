# qingbird Agent Identity & Subagent Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修正 v0.3.0 的 agent 身份定位（system prompt + nudge 文案），并彻底重做 subagent 系统（当前是孤儿 skeleton 代码）。重做后 subagent 通过 `delegate_task` 工具被 LLM 主动调用，profile 系统支持内置 + yaml 用户扩展，子会话持久化到 SessionStore，**为 v0.4 进化系统（CompactionManager / Reflection Engine / Profile Compilation）预留 SubagentExecutor 作为通用管道**。

**Architecture:** PR-A 改提示词文案（无架构变更）。PR-B1 引入 `SubagentProfile` 数据模型 + 内置字典 + yaml 加载 + 合并。PR-B2 扩展 SessionStore schema（`relation` / `parent_session_id` / `role` 字段），实现 `SubagentExecutor`（管子 agent 生命周期），实现 `DelegateTaskTool`（LLM 主动调用），接入 main.rs。设计参考 `F:\AI\Kun\kun\src\adapters\tool\delegation-tool-provider.ts` + `F:\AI\Kun\kun\src\delegation/builtin-profiles.ts`。

**Tech Stack:** Rust 2024, tokio, serde, serde_yaml, rusqlite (bundled), async-trait, `rust-i18n`

**参考文档：**
- `F:\AI\Kun\kun\src\prompt\kun-system-prompt.ts` — system prompt 范本
- `F:\AI\Kun\kun\src\adapters\tool\delegation-tool-provider.ts` — delegate_task 工具设计
- `F:\AI\Kun\kun\src\delegation\builtin-profiles.ts` — 内置 profile 字典
- `F:\AI\Kun\kun\src\delegation\child-agent-executor.ts` — 子 agent 生命周期
- `F:\AI\agent-evolution-with-hermes.md` — v0.4 进化系统设计（PR-B 预留钩子的依据）

---

## Global Constraints

从 `AGENTS.md` + 现有 `v0.3 plan` 抄录 + 本次新增：

- 所有用户面向字符串走 `rust_i18n::t!()`，`locales/zh-CN.yml` 和 `locales/en-US.yml` 必须同时新增/修改
- 代码注释保持英文，`tracing` 日志保持英文
- 4 门禁 PR 前必过：`cargo fmt --check` → `cargo clippy --all-targets --workspace -- -D warnings` → `cargo test --workspace` → `cargo build`
- 依赖方向严格：`qbird-code-models ← qbird-code-infra ← {qbird-code-tools, qbird-code-agents} ← qbird-code`，下层禁止引用上层
- Conventional Commits，scope = 模块名（`agents` / `tools` / `cli` / `i18n` / `infra` / `chore` / `docs`）
- 涉及 LLM 的测试必须 dummy key + 5s timeout
- 本计划不跳版本号：v0.3.0 已发布 → 本计划版本为 **v0.3.1**
- 共享 workspace 版本号，所有 crate 同步 bump 到 0.3.1
- 现有 `Subagent` / `SubagentConfig` / `SubagentRole` / `execute_parallel` 仅在 `qbird-code-agents/src/{subagent,subagent_pool,lib}.rs` 内部引用，无外部用户 — 可安全整体替换
- 现有 sample profile（`developer` / `researcher`）由 `Profile::create_sample_profiles` 创建，硬编码在 `crates/qbird-code-infra/src/profile.rs:132-150`

---

## 文件结构总览

| 操作 | 文件 | 涉及 PR | 用途 |
|---|---|---|---|
| **删除** | `crates/qbird-code-agents/src/subagent.rs` | PR-B1 | 孤儿 Subagent struct |
| **删除** | `crates/qbird-code-agents/src/subagent_pool.rs` | PR-B1 | 孤儿 execute_parallel |
| **新建** | `crates/qbird-code-agents/src/subagent/mod.rs` | PR-B1 | pub use |
| **新建** | `crates/qbird-code-agents/src/subagent/profile.rs` | PR-B1 | `SubagentProfile` struct + 内置字典 |
| **新建** | `crates/qbird-code-agents/src/subagent/config.rs` | PR-B1 | yaml 加载 + 合并 |
| **新建** | `crates/qbird-code-agents/src/subagent/executor.rs` | PR-B2 | `SubagentExecutor` + `ChildRecord` |
| **新建** | `crates/qbird-code-tools/src/delegate_task.rs` | PR-B2 | `DelegateTaskTool`（实现 `Tool` trait） |
| **新建** | `crates/qbird-code-agents/src/subagent/profile_test.rs` | PR-B1 | 单元测试 |
| **新建** | `crates/qbird-code-agents/src/subagent/config_test.rs` | PR-B1 | 单元测试 |
| **新建** | `crates/qbird-code-agents/src/subagent/executor_test.rs` | PR-B2 | 单元测试 |
| **新建** | `crates/qbird-code-tools/src/delegate_task_test.rs` | PR-B2 | 单元测试 |
| **新建** | `crates/qbird-code-tools/tests/delegate_task_integration_test.rs` | PR-B2 | mock executor 集成测试 |
| **修改** | `crates/qbird-code-agents/src/lib.rs` | PR-B1, PR-B2 | 替换 exports |
| **修改** | `crates/qbird-code-tools/src/lib.rs` | PR-B2 | `pub use delegate_task::DelegateTaskTool;` |
| **修改** | `crates/qbird-code-infra/src/memory/session_store.rs` | PR-B2 | schema 迁移 + `list_sessions(include_side)` |
| **修改** | `crates/qbird-code-infra/src/profile.rs` | PR-A | sample profile 提示词同步 |
| **修改** | `crates/qbird-code/src/main.rs` | PR-A, PR-B2 | system prompt 来源切换 + `DelegateTaskTool` 注册 |
| **修改** | `crates/qbird-code-agents/src/react_loop/types.rs` | PR-B2 | `ReactLoopConfig.subagent_executor` 字段 |
| **修改** | `crates/qbird-code-agents/src/react_loop/mod.rs` | PR-B2 | `execute_tools_*` 加 delegate_task 分支 |
| **修改** | `crates/qbird-code-models/src/error.rs` | PR-B1, PR-B2 | `SubagentProfileNotFound` variant |
| **修改** | `locales/zh-CN.yml` | PR-A, PR-B1, PR-B2 | system_prompt + nudge + subagent i18n |
| **修改** | `locales/en-US.yml` | PR-A, PR-B1, PR-B2 | 同上（对称） |
| **修改** | `Cargo.toml`（workspace） | PR-B1, PR-B2 | bump 到 0.3.1 |
| **修改** | `CHANGELOG.md` | PR-A, PR-B1, PR-B2 | v0.3.1 段 |
| **修改** | `CLAUDE.md` | PR-B2 | 状态同步到 v0.3.1 |
| **修改** | `docs/cli.md` | PR-B2 | 加 subagent / delegate_task 章节 |

---

## 主计划表（Phased Implementation Plan Table）

| PR | 标题 | Scope | Test | Risk | Dep | Est |
|---|---|---|---|---|---|---|
| **PR-A** | Agent 身份 + Nudge 重写 | 改 system_prompt + 4 个 nudge + 2 个 sample profile | 4 门禁 + 现有 357 测试 | 低 | – | 0.5d |
| **PR-B1** | Subagent profile 系统 | 删孤儿 subagent.rs；建 `SubagentProfile` 数据模型 + 内置字典（5 个） + yaml 加载合并 + 单元测试 | 14+ 单元测试 | 中 | PR-A | 1.5d |
| **PR-B2** | Subagent executor + delegate_task 工具 | SessionStore schema 迁移 + `SubagentExecutor` + `DelegateTaskTool` + main.rs 接线 + ReactLoop 集成 + 集成测试 | 12+ 单元 + 2 集成 | 中 | PR-B1 | 2-3d |

**合计：** 3 PR，~4-5 工作日，1 周。

---

## PR-A — Agent 身份 + Nudge 重写

### Task A1: 重写 zh-CN.yml 的 system_prompt

**Files:**
- Modify: `locales/zh-CN.yml:146` （`system_prompt` 键）

- [ ] **Step 1: 修改 system_prompt 字段**

将 `locales/zh-CN.yml` 第 146 行的 `system_prompt` 字段完整替换为：

```yaml
system_prompt: |
  你是 qingbird，一个能帮助用户完成多种任务的助手。当前 Provider：%{provider}。
  可用工具：%{tools}

  核心原则：
  - 先理解用户意图，再决定如何回应
  - 意图清晰就行动；意图不清时礼貌确认或询问
  - 用户没说要做编码任务前，不假设任何工作上下文
  - 工具仅在相关且必要时调用；闲聊时不要主动提及工具
  - 直接、简洁、有用；不要客套话或自我推销
  - 中文对话用中文回复
```

- [ ] **Step 2: 验证编译**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo build -p qbird-code
```

Expected: 编译成功（rust-i18n 编译时加载 yaml）

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add locales/zh-CN.yml && git commit -m "i18n(zh-CN): 重写 system_prompt 提示词

去掉'编码助手'身份定位，按 Kun 范本改为中性多面手助手。
明确'先理解意图、清晰就行动、不清就询问'的核心行为原则。"
```

---

### Task A2: 重写 en-US.yml 的 system_prompt

**Files:**
- Modify: `locales/en-US.yml:144` （`system_prompt` 键）

- [ ] **Step 1: 修改 system_prompt 字段**

将 `locales/en-US.yml` 第 144 行的 `system_prompt` 字段完整替换为：

```yaml
system_prompt: |
  You are qingbird, a helpful assistant capable of supporting users across multiple domains. Current Provider: %{provider}.
  Available tools: %{tools}

  Core principles:
  - Understand the user's intent first, then decide how to respond
  - Act when intent is clear; ask politely when intent is unclear
  - Don't assume a coding/work context unless the user explicitly says so
  - Use tools only when relevant and necessary; do not volunteer tool usage during chitchat
  - Be direct, concise, and useful; no performative filler or self-promotion
  - Reply in the same language as the user
```

- [ ] **Step 2: 验证编译**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo build -p qbird-code
```

Expected: 编译成功

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add locales/en-US.yml && git commit -m "i18n(en-US): 重写 system_prompt 提示词

跟 zh-CN 同步，按 Kun 范本改为中性多面手助手。"
```

---

### Task A3: 重写 zh-CN.yml 的 4 个 nudge 文案

**Files:**
- Modify: `locales/zh-CN.yml:104-108` （Nudge 段）

- [ ] **Step 1: 修改 nudge 字段**

将 `locales/zh-CN.yml` 第 104-108 行的 4 个 nudge 键完整替换为：

```yaml
nudge_consecutive_reads: "你已经连续 %{count} 轮只进行读取操作。是否需要调整当前策略？"
nudge_iteration_warning: "只剩 %{remaining} 轮迭代机会了。请尽快收敛，给出最终结论。"
nudge_completion_without_write: "你声明了任务完成，但尚未执行任何写入操作。如果有实际改动未完成，请先完成；如果是纯研究/咨询类任务，则无需写入。"
nudge_no_tool_calls: "你已连续多轮没有调用工具。如果有可用的工具能推进任务，请使用；如果是纯对话则无需工具。"
```

- [ ] **Step 2: 验证**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-agents nudge
```

Expected: 现有 9 个 nudge 单元测试 PASS（仅文案变化不影响逻辑）

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add locales/zh-CN.yml && git commit -m "i18n(zh-CN): 重写 4 个 nudge 文案

去掉'是否要写代码'假设，改为中性'是否需要调整策略'。
给纯研究/纯对话场景提供出口。"
```

---

### Task A4: 重写 en-US.yml 的 4 个 nudge 文案

**Files:**
- Modify: `locales/en-US.yml:102-106` （Nudge reminders 段）

- [ ] **Step 1: 修改 nudge 字段**

将 `locales/en-US.yml` 第 102-106 行的 4 个 nudge 键完整替换为：

```yaml
nudge_consecutive_reads: "You have performed %{count} consecutive read-only operations. Do you need to adjust your current strategy?"
nudge_iteration_warning: "Only %{remaining} iterations remaining. Please converge and give a final answer."
nudge_completion_without_write: "You declared the task complete, but no write operations were performed. Finish any actual changes first; if this was a pure research or consultation task, no writes are needed."
nudge_no_tool_calls: "You have gone multiple rounds without calling any tool. Use available tools to advance the task if applicable; for pure conversation, no tools are needed."
```

- [ ] **Step 2: 验证**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-agents nudge
```

Expected: 9 个 nudge 测试 PASS

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add locales/en-US.yml && git commit -m "i18n(en-US): 重写 4 个 nudge 文案

跟 zh-CN 同步。"
```

---

### Task A5: 同步更新 sample profile `developer`

**Files:**
- Modify: `crates/qbird-code-infra/src/profile.rs:132-138` （`developer_yaml` 字符串字面量）

- [ ] **Step 1: 修改 developer profile 的 system_prompt**

将 `developer_yaml` 字面量中第 134 行：

```yaml
system_prompt: "你是一个专业的 Rust 开发助手。使用中文回复，代码注释保持英文。"
```

替换为：

```yaml
system_prompt: |
  你是一个 Rust 开发助手，专注于帮助用户编写、审查和改进 Rust 代码。

  工作方式：
  - 先理解用户的具体需求和约束，再给出方案
  - 尊重现有代码风格和约定，不做大范围重写
  - 读当前状态再行动；不确定时询问或检查文件
  - 使用中文回复，代码注释保持英文
```

- [ ] **Step 2: 验证**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-infra sample_profiles
```

Expected: 现有 sample profile 测试 PASS

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-infra/src/profile.rs && git commit -m "infra(profile): 更新 sample 'developer' 提示词

跟主 system_prompt 改写对齐。"
```

---

### Task A6: 同步更新 sample profile `researcher`

**Files:**
- Modify: `crates/qbird-code-infra/src/profile.rs:140-150` （`researcher_yaml` 字符串字面量）

- [ ] **Step 1: 修改 researcher profile 的 system_prompt**

将 `researcher_yaml` 字面量中第 142 行：

```yaml
system_prompt: "你是一个研究助手，专注于信息检索和分析。只使用只读工具。"
```

替换为：

```yaml
system_prompt: |
  你是一个研究助手，专注于信息检索、整合与分析。
  只使用只读工具收集信息，不修改任何文件；找到答案后清晰汇报发现。
```

- [ ] **Step 2: 验证**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-infra sample_profiles
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-infra/src/profile.rs && git commit -m "infra(profile): 更新 sample 'researcher' 提示词"
```

---

### Task A7: 跑 4 门禁 + 更新 CHANGELOG

**Files:**
- Modify: `CHANGELOG.md` （`[Unreleased]` 段）

- [ ] **Step 1: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 2: 在 CHANGELOG.md 的 `[Unreleased]` 段加条目**

```markdown
### Changed
- **agent 身份定位**：重写 system_prompt（zh-CN + en-US），去掉"编码助手"身份，改为中性多面手助手。
- **nudge 文案**：重写 4 个 nudge 提示（zh-CN + en-US），去掉"是否要写代码"假设。
- **sample profile 提示词**：同步更新 `developer` 和 `researcher` 的 system_prompt。
```

- [ ] **Step 3: Commit + Push**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add CHANGELOG.md && git commit -m "chore: CHANGELOG 记录 PR-A 改动"
git push
```

---

## PR-B1 — Subagent Profile 系统

> 前置：PR-A 已合入

### Task B1-1: 删除孤儿 subagent.rs 和 subagent_pool.rs

**Files:**
- Delete: `crates/qbird-code-agents/src/subagent.rs`
- Delete: `crates/qbird-code-agents/src/subagent_pool.rs`
- Modify: `crates/qbird-code-agents/src/lib.rs:7-12`

- [ ] **Step 1: 删除两个孤儿文件**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && rm crates/qbird-code-agents/src/subagent.rs crates/qbird-code-agents/src/subagent_pool.rs
```

- [ ] **Step 2: 修改 lib.rs**

将 `crates/qbird-code-agents/src/lib.rs` 替换为：

```rust
rust_i18n::i18n!("../../locales", fallback = "en-US");

pub mod doom_loop;
pub mod nudge;
pub mod react_loop;
pub mod skill;
pub mod subagent;

pub use react_loop::{ReactLoop, ReactLoopConfig};
pub use subagent::{
    ChildRecord, ChildStatus, SubagentExecutor, SubagentMode, SubagentProfile,
    SubagentProfileConfig, SubagentSpawnHints, ToolPolicy,
};
```

注意：`pub mod subagent;` 引用 `subagent/mod.rs`，B1-2 会创建。

- [ ] **Step 3: 创建临时 placeholder 让 lib.rs 编译过**

创建 `crates/qbird-code-agents/src/subagent/mod.rs`（B1-2 会替换为完整内容）：

```rust
//! Subagent placeholder — B1-2 完整实现
pub mod profile;
```

创建 `crates/qbird-code-agents/src/subagent/profile.rs`（B1-2 会替换为完整内容）：

```rust
//! Placeholder — B1-2 完整实现
pub use std::collections::HashMap;
```

- [ ] **Step 4: 验证编译**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo build -p qbird-code-agents
```

Expected: 编译成功（placeholder 编译过）

- [ ] **Step 5: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add -A crates/qbird-code-agents/src/ && git commit -m "agents(subagent): 删除孤儿 subagent.rs 和 subagent_pool.rs

当前 subagent 模块只是骨架代码，未被任何代码引用。
PR-B1 整体重做时彻底替换。"
```

---

### Task B1-2: 创建 `SubagentProfile` 数据模型 + 5 个内置 profile

**Files:**
- Modify: `crates/qbird-code-agents/src/subagent/mod.rs` （替换 placeholder）
- Modify: `crates/qbird-code-agents/src/subagent/profile.rs` （替换 placeholder）
- Create: `crates/qbird-code-agents/src/subagent/profile_test.rs`
- Create: `crates/qbird-code-agents/src/subagent/config.rs` （占位）
- Create: `crates/qbird-code-agents/src/subagent/executor.rs` （占位）

- [ ] **Step 1: 替换 `subagent/mod.rs`**

```rust
//! Subagent 系统：让 LLM 主动派发子任务给独立 ReAct 循环实例。
//!
//! 设计参考 `F:\AI\Kun\kun\src\delegation/`：
//! - `profile.rs`    — `SubagentProfile` 数据模型 + 内置字典
//! - `config.rs`     — yaml 加载 + 与内置合并
//! - `executor.rs`   — 子 agent 生命周期管理（PR-B2）
//!
//! 这个模块是 v0.4 进化系统（CompactionManager / Reflection Engine /
//! Profile Compilation）的通用管道；预留 `model` 字段和 `SubagentSpawnHints`
//! 让这些 feature 接入时零摩擦。

pub mod config;
pub mod executor;
pub mod profile;

pub use config::{load_profiles, SubagentProfileConfig};
pub use executor::{ChildEvent, ChildRecord, ChildStatus, SpawnPriority, SubagentExecutor, SubagentSpawnHints};
pub use profile::{builtin_profiles, SubagentMode, SubagentProfile, ToolPolicy};
```

- [ ] **Step 2: 替换 `subagent/profile.rs`**

```rust
//! `SubagentProfile` 数据模型 + 内置 profile 字典。
//!
//! 一个 profile = 一个子 agent 角色定义（系统提示词前置段 + 工具策略 +
//! 描述 + 默认工具集）。LLM 通过 `delegate_task` 工具按 profile 名字
//! 派发子任务。

use serde::{Deserialize, Serialize};

/// 工具策略：决定子 agent 可用工具集。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPolicy {
    /// 只读工具集
    ReadOnly,
    /// 继承父 agent 的完整工具集
    Inherit,
}

/// Subagent 模式（预留字段；v0.3.1 只用 Subagent）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentMode {
    /// 子 agent：独立 ReAct 循环实例，独立 session
    Subagent,
    /// 主 agent（预留：v0.4+ 启动多 persona 时使用）
    Primary,
}

/// Subagent profile 配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentProfile {
    pub name: String,
    pub mode: SubagentMode,
    pub tool_policy: ToolPolicy,
    pub prompt_preamble: String,
    pub description: String,
    pub default_tools: Vec<String>,
    pub max_iterations: Option<usize>,
    /// 预留：v0.4 reflection / compilation 用小模型时覆盖
    pub model: Option<String>,
}

impl SubagentProfile {
    pub fn read_only_tool_names() -> &'static [&'static str] {
        &["read_file", "search_code", "glob", "list_dir", "web_fetch"]
    }
}

/// 5 个内置 profile（Kun `BUILTIN_SUBAGENT_PROFILES` 对齐）
pub fn builtin_profiles() -> Vec<SubagentProfile> {
    vec![
        SubagentProfile {
            name: "general".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::Inherit,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「通用代理」(General)。",
                "你能研究复杂问题并执行多步骤任务，拥有与主代理一致的完整工具访问权限。",
                "适合被派去并行承担一个独立的工作单元。",
                "聚焦交给你的具体任务，完成后简洁汇报结果与关键改动。",
            ).into(),
            description: "通用代理：研究复杂问题、执行多步骤任务，可读写文件、运行命令。".into(),
            default_tools: vec![],
            max_iterations: None,
            model: None,
        },
        SubagentProfile {
            name: "explore".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「探索代理」(Explore)，一个快速的只读代码库代理。",
                "你只读取/搜索/列目录/抓网页，绝不修改任何文件。",
                "当需要按模式快速查找文件、搜索代码关键字、或回答关于代码库的问题时使用你。",
                "高效定位相关位置，返回结论（文件:行 + 简要说明），不做与任务无关的展开。",
            ).into(),
            description: "只读探索代理：快速查找文件、搜索关键字、回答关于代码库的问题。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter().map(|s| s.to_string()).collect(),
            max_iterations: Some(15),
            model: None,
        },
        SubagentProfile {
            name: "code-writer".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::Inherit,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「代码编写代理」(Code Writer)。",
                "你专注于实现具体功能/修复 bug：读相关代码、设计方案、写入修改、运行测试验证。",
                "尊重现有代码风格、命名约定和依赖选型；不引入未经用户同意的新依赖。",
                "完成时汇报：改了哪些文件、关键设计决策、是否有未验证的风险。",
            ).into(),
            description: "代码编写代理：实现功能/修 bug，读写文件、运行测试。".into(),
            default_tools: vec![],
            max_iterations: None,
            model: None,
        },
        SubagentProfile {
            name: "planner".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「规划代理」(Planner)，一个纯推理规划角色。",
                "你不修改任何文件，只读代码、思考、设计。",
                "产出物：分步骤的实施计划，每步标明（输入/动作/输出/风险），",
                "识别依赖关系和可并行的工作单元。",
                "计划要具体到文件:函数级别，避免泛泛而谈。",
            ).into(),
            description: "规划代理：纯推理设计实施方案，不修改文件。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter().map(|s| s.to_string()).collect(),
            max_iterations: Some(10),
            model: None,
        },
        SubagentProfile {
            name: "reviewer".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「审查代理」(Reviewer)。",
                "你只读代码并报告问题，不做任何修改。",
                "审查维度：正确性（含边界情况）、错误处理、可读性、与现有约定的一致性。",
                "每条问题给出：文件:行 + 问题描述 + 严重程度（critical/major/minor）+ 建议改法。",
                "按严重程度排序输出，不要泛泛而谈。",
            ).into(),
            description: "审查代理：只读代码审查问题，不修改。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter().map(|s| s.to_string()).collect(),
            max_iterations: Some(12),
            model: None,
        },
    ]
}
```

- [ ] **Step 3: 创建 `subagent/config.rs`（占位 + 简单 load）**

```rust
//! yaml 加载 + 与内置 profile 合并。
//!
//! 用户 yaml 格式：
//! ```yaml
//! subagents:
//!   profiles:
//!     general:
//!       prompt_preamble: "..."
//!       max_iterations: 30
//!     my-custom:
//!       prompt_preamble: "..."
//!       tool_policy: readonly
//! ```

use std::collections::HashMap;

use qbird_code_models::{EflowError, Result};
use serde::Deserialize;

use super::profile::{SubagentMode, SubagentProfile, ToolPolicy};

#[derive(Debug, Deserialize)]
struct SubagentsConfig {
    #[serde(default)]
    subagents: SubagentsSection,
}

#[derive(Debug, Default, Deserialize)]
struct SubagentsSection {
    #[serde(default)]
    profiles: Vec<SubagentProfileConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentProfileConfig {
    pub name: String,
    pub mode: Option<SubagentMode>,
    pub tool_policy: Option<ToolPolicy>,
    pub prompt_preamble: Option<String>,
    pub description: Option<String>,
    pub default_tools: Option<Vec<String>>,
    pub max_iterations: Option<usize>,
    pub model: Option<String>,
}

/// 从 yaml 文本加载并与内置合并
pub fn load_profiles_from_yaml(yaml_text: Option<&str>) -> Result<HashMap<String, SubagentProfile>> {
    let mut map: HashMap<String, SubagentProfile> = super::profile::builtin_profiles()
        .into_iter().map(|p| (p.name.clone(), p)).collect();

    if let Some(text) = yaml_text {
        let config: SubagentsConfig = serde_yaml::from_str(text).map_err(|e| {
            EflowError::Internal(format!("subagent yaml 解析失败: {}", e))
        })?;
        merge_into_builtins(&mut map, &config.subagents.profiles);
    }

    Ok(map)
}

/// 顶层加载入口
pub fn load_profiles(yaml_text: Option<&str>) -> Result<HashMap<String, SubagentProfile>> {
    load_profiles_from_yaml(yaml_text)
}

/// 用户配置逐字段覆盖 builtin（None 字段保留 builtin 值）
pub fn merge_into_builtins(
    map: &mut HashMap<String, SubagentProfile>,
    user_configs: &[SubagentProfileConfig],
) {
    for cfg in user_configs {
        if let Some(builtin) = map.get(&cfg.name).cloned() {
            let merged = SubagentProfile {
                name: builtin.name.clone(),
                mode: cfg.mode.unwrap_or(builtin.mode),
                tool_policy: cfg.tool_policy.unwrap_or(builtin.tool_policy),
                prompt_preamble: cfg.prompt_preamble.clone().unwrap_or(builtin.prompt_preamble),
                description: cfg.description.clone().unwrap_or(builtin.description),
                default_tools: cfg.default_tools.clone().unwrap_or(builtin.default_tools),
                max_iterations: cfg.max_iterations.or(builtin.max_iterations),
                model: cfg.model.clone().or(builtin.model),
            };
            map.insert(cfg.name.clone(), merged);
        } else {
            let new_profile = SubagentProfile {
                name: cfg.name.clone(),
                mode: cfg.mode.unwrap_or(SubagentMode::Subagent),
                tool_policy: cfg.tool_policy.unwrap_or(ToolPolicy::Inherit),
                prompt_preamble: cfg.prompt_preamble.clone().unwrap_or_default(),
                description: cfg.description.clone().unwrap_or_default(),
                default_tools: cfg.default_tools.clone().unwrap_or_default(),
                max_iterations: cfg.max_iterations,
                model: cfg.model.clone(),
            };
            map.insert(cfg.name.clone(), new_profile);
        }
    }
}
```

- [ ] **Step 4: 创建 `subagent/executor.rs`（占位 + 类型定义）**

```rust
//! `SubagentExecutor` — 子 agent 生命周期管理（PR-B2 完整实现）

use std::sync::Arc;

use qbird_code_models::{EflowError, UsageStats};

use super::profile::ToolPolicy;

#[derive(Debug, Clone, Default)]
pub struct SubagentSpawnHints {
    pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>,
    /// v0.3.1 固定 false
    pub detached: bool,
    /// v0.3.1 固定 Normal
    pub priority: SpawnPriority,
    /// v0.4+ 事件回调
    pub on_event: Option<Arc<dyn Fn(ChildEvent) + Send + Sync>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpawnPriority {
    #[default]
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub enum ChildEvent {
    Started { child_id: String },
    Completed { summary: String, usage: UsageStats },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct ChildRecord {
    pub child_id: String,
    pub status: ChildStatus,
    pub summary: String,
    pub usage: UsageStats,
    pub profile: String,
    pub tool_policy: ToolPolicy,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Completed,
    Failed,
}

pub struct SubagentExecutor {
    pub(crate) _placeholder: (),
}

impl SubagentExecutor {
    pub fn placeholder() -> Self {
        Self { _placeholder: () }
    }

    /// 临时：返回 IO error 让编译过
    pub fn list_profile_names(&self) -> Vec<String> {
        vec![]
    }
    pub fn validate_profile(&self, _name: &str) -> Result<&super::profile::SubagentProfile, EflowError> {
        Err(EflowError::Internal("placeholder".into()))
    }
}
```

- [ ] **Step 5: 创建 `subagent/profile_test.rs`**

```rust
use crate::subagent::profile::{
    builtin_profiles, SubagentMode, SubagentProfile, ToolPolicy,
};

#[test]
fn builtin_profiles_returns_5_entries() {
    assert_eq!(builtin_profiles().len(), 5);
}

#[test]
fn builtin_profiles_have_unique_names() {
    let mut names: Vec<&str> = builtin_profiles().iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 5);
}

#[test]
fn builtin_general_uses_inherit_policy() {
    let general = builtin_profiles().into_iter()
        .find(|p| p.name == "general").expect("general must exist");
    assert_eq!(general.tool_policy, ToolPolicy::Inherit);
    assert_eq!(general.mode, SubagentMode::Subagent);
    assert!(general.model.is_none());
}

#[test]
fn builtin_explore_uses_readonly_with_default_tools() {
    let explore = builtin_profiles().into_iter()
        .find(|p| p.name == "explore").expect("explore must exist");
    assert_eq!(explore.tool_policy, ToolPolicy::ReadOnly);
    assert!(explore.default_tools.contains(&"read_file".to_string()));
    assert!(explore.max_iterations.is_some());
}

#[test]
fn builtin_planner_and_reviewer_are_readonly() {
    let profiles = builtin_profiles();
    for name in ["planner", "reviewer"] {
        let p = profiles.iter().find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{} must exist", name));
        assert_eq!(p.tool_policy, ToolPolicy::ReadOnly, "{} should be read-only", name);
    }
}

#[test]
fn read_only_tool_names_contains_expected_tools() {
    let names = SubagentProfile::read_only_tool_names();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"search_code"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"list_dir"));
    assert!(names.contains(&"web_fetch"));
}
```

- [ ] **Step 6: 跑测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-agents subagent::profile
```

Expected: 6 个单元测试 PASS

- [ ] **Step 7: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 8: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add -A crates/qbird-code-agents/src/subagent/ && git add crates/qbird-code-agents/src/lib.rs && git commit -m "agents(subagent): 新建 SubagentProfile 数据模型 + 5 个内置 profile

- SubagentProfile 包含 mode / tool_policy / prompt_preamble / description
  / default_tools / max_iterations / model（预留 v0.4 进化系统）
- 5 个内置 profile：general / explore / code-writer / planner / reviewer
- 预留 SubagentSpawnHints / ChildEvent 接口供 v0.4 接入
- 6 个单元测试覆盖内置 profile"
```

---

### Task B1-3: 实现 yaml 加载 + 合并逻辑

**Files:**
- Modify: `crates/qbird-code-agents/src/subagent/config.rs` （B1-2 已写，本任务添加测试）

- [ ] **Step 1: 创建 `subagent/config_test.rs`**

```rust
use std::collections::HashMap;

use crate::subagent::config::{
    load_profiles_from_yaml, merge_into_builtins, SubagentProfileConfig,
};
use crate::subagent::profile::{builtin_profiles, ToolPolicy};

#[test]
fn load_profiles_from_yaml_empty_returns_builtins() {
    let map = load_profiles_from_yaml(None).expect("no yaml");
    let builtins = builtin_profiles();
    assert_eq!(map.len(), builtins.len());
    for b in builtins {
        assert!(map.contains_key(&b.name), "missing builtin {}", b.name);
    }
}

#[test]
fn load_profiles_from_yaml_user_override_replaces_field() {
    let yaml = r#"
profiles:
  general:
    prompt_preamble: "覆盖后的提示词"
    max_iterations: 30
"#;
    let map = load_profiles_from_yaml(Some(yaml)).expect("parse yaml");
    let general = map.get("general").expect("general exists");
    assert_eq!(general.prompt_preamble, "覆盖后的提示词");
    assert_eq!(general.max_iterations, Some(30));
    assert_eq!(general.tool_policy, ToolPolicy::Inherit);
}

#[test]
fn load_profiles_from_yaml_user_adds_new_profile() {
    let yaml = r#"
profiles:
  my-custom:
    prompt_preamble: "自定义代理"
    tool_policy: readonly
    description: "测试"
"#;
    let map = load_profiles_from_yaml(Some(yaml)).expect("parse yaml");
    let custom = map.get("my-custom").expect("my-custom exists");
    assert_eq!(custom.prompt_preamble, "自定义代理");
    assert_eq!(custom.tool_policy, ToolPolicy::ReadOnly);
    assert!(map.contains_key("general"));
    assert!(map.contains_key("explore"));
}

#[test]
fn load_profiles_malformed_yaml_returns_error() {
    let yaml = "this is: not: valid: yaml: [[[";
    let result = load_profiles_from_yaml(Some(yaml));
    assert!(result.is_err());
}

#[test]
fn merge_into_builtins_user_wins_per_field() {
    let mut map: HashMap<String, crate::subagent::profile::SubagentProfile> = builtin_profiles()
        .into_iter().map(|p| (p.name.clone(), p)).collect();
    let user_config = SubagentProfileConfig {
        name: "general".into(),
        mode: None,
        tool_policy: Some(ToolPolicy::ReadOnly),
        prompt_preamble: Some("新提示".into()),
        description: None,
        default_tools: None,
        max_iterations: Some(20),
        model: None,
    };
    merge_into_builtins(&mut map, &[user_config]);
    let g = map.get("general").unwrap();
    assert_eq!(g.tool_policy, ToolPolicy::ReadOnly);
    assert_eq!(g.prompt_preamble, "新提示");
    assert_eq!(g.max_iterations, Some(20));
    assert!(g.description.contains("通用代理"));
}
```

- [ ] **Step 2: 跑测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-agents subagent::config
```

Expected: 5 个测试 PASS

- [ ] **Step 3: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-agents/src/subagent/config_test.rs && git commit -m "agents(subagent): 加 yaml 加载 + 合并的单元测试

- 5 个测试覆盖空 yaml / 字段覆盖 / 新增 profile / 解析失败 / merge 行为
- 实现代码在 B1-2 的 config.rs 一次性写好"
```

---

### Task B1-4: EflowError 加 SubagentProfileNotFound 变体

**Files:**
- Modify: `crates/qbird-code-models/src/error.rs` （添加 variant + user_message）
- Modify: `locales/zh-CN.yml` （i18n key）
- Modify: `locales/en-US.yml` （i18n key）

- [ ] **Step 1: 找到现有 EflowError 枚举**

读 `crates/qbird-code-models/src/error.rs` 找到 `EflowError` 枚举定义位置。

- [ ] **Step 2: 添加新 variant**

在 `EflowError` 枚举中加：

```rust
/// Subagent profile 未找到
#[error("subagent profile not found: {name}")]
SubagentProfileNotFound { name: String },
```

- [ ] **Step 3: 在 `user_message()` 方法中加对应分支**

```rust
EflowError::SubagentProfileNotFound { name } => {
    rust_i18n::t!("err_subagent_profile_not_found", name = name.as_str()).to_string()
}
```

- [ ] **Step 4: 加 i18n key**

`locales/zh-CN.yml` 加：

```yaml
err_subagent_profile_not_found: "未找到 subagent profile: %{name}"
err_subagent_policy_denied: "profile '%{profile}' 是 %{policy}，不能调用工具 %{tool}"
err_subagent_execution_failed: "subagent 执行失败: %{error}"
```

`locales/en-US.yml` 加：

```yaml
err_subagent_profile_not_found: "Subagent profile not found: %{name}"
err_subagent_policy_denied: "profile '%{profile}' is %{policy}, cannot call tool %{tool}"
err_subagent_execution_failed: "subagent execution failed: %{error}"
```

- [ ] **Step 5: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 6: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-models/src/error.rs locales/zh-CN.yml locales/en-US.yml && git commit -m "models: EflowError 加 SubagentProfileNotFound variant

预留 SubagentPolicyDenied 和 SubagentExecutionFailed i18n key 给 PR-B2 用。"
```

---

### Task B1-5: 跑 4 门禁 + CHANGELOG

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `Cargo.toml` （workspace 版本 bump）

- [ ] **Step 1: Bump workspace 版本**

读 `Cargo.toml`（workspace 根），找到 `workspace.package.version`，bump 到 `0.3.1`。

- [ ] **Step 2: 在 CHANGELOG.md 加 v0.3.1 段**

```markdown
## [0.3.1] - 2026-06-XX

### Added
- **Subagent profile 系统**：`SubagentProfile` 数据模型 + 5 个内置 profile
  (general / explore / code-writer / planner / reviewer) + yaml 用户扩展逻辑。
  预留 `SubagentSpawnHints` / `ChildEvent` 接口供 v0.4 进化系统接入。

### Changed
- 旧 `Subagent` / `SubagentConfig` / `SubagentRole` / `execute_parallel`
  孤儿代码删除（PR-B1 整体重做；B2 接 delegate_task 工具）。
```

- [ ] **Step 3: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 4: Commit + Push**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add Cargo.toml CHANGELOG.md && git commit -m "chore: bump v0.3.1 + CHANGELOG"
git push
```

---

## PR-B2 — Subagent Executor + DelegateTaskTool

> 前置：PR-B1 已合入

### Task B2-1: SessionStore schema 扩展

**Files:**
- Modify: `crates/qbird-code-infra/src/memory/session_store.rs` （`open()` + 新增方法）

- [ ] **Step 1: 改写 `open()` 的 schema 初始化**

将 `crates/qbird-code-infra/src/memory/session_store.rs:26-46` 的 `execute_batch` 改为：

```rust
conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        name TEXT DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        message_count INTEGER DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS session_messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        FOREIGN KEY (session_id) REFERENCES sessions(id)
    );
    CREATE TABLE IF NOT EXISTS meta (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
)
.map_err(|e| EflowError::Memory(format!("Failed to create session tables: {}", e)))?;

// v0.3.1 迁移：sessions 表加 relation / parent_session_id / role
let columns: Vec<String> = conn
    .prepare("PRAGMA table_info(sessions)")
    .map_err(|e| EflowError::Memory(e.to_string()))?
    .query_map([], |row| row.get::<_, String>(1))
    .map_err(|e| EflowError::Memory(e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

if !columns.iter().any(|c| c == "relation") {
    conn.execute_batch(
        "ALTER TABLE sessions ADD COLUMN relation TEXT NOT NULL DEFAULT 'main';
         ALTER TABLE sessions ADD COLUMN parent_session_id TEXT;
         ALTER TABLE sessions ADD COLUMN role TEXT;",
    )
    .map_err(|e| EflowError::Memory(format!("Failed to migrate sessions schema: {}", e)))?;
}
```

- [ ] **Step 2: 编译验证**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo build -p qbird-code-infra
```

Expected: 编译成功

- [ ] **Step 3: 跑现有 SessionStore 测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-infra session
```

Expected: 现有 session 测试全部 PASS

- [ ] **Step 4: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-infra/src/memory/session_store.rs && git commit -m "infra(memory): SessionStore 加 relation / parent_session_id / role 字段

迁移逻辑：检查列存在后才 ALTER（向后兼容老库）。
v0.3.1 subagent 持久化用，v0.4 进化系统复用。"
```

---

### Task B2-2: SessionStore API 扩展

**Files:**
- Modify: `crates/qbird-code-infra/src/memory/session_store.rs`

- [ ] **Step 1: 加新方法签名**

在 `impl SessionStore` 块内加：

```rust
/// 列出 session；`include_side=true` 时包含 subagent 子会话
pub fn list_sessions_filtered(
    &self,
    include_side: bool,
) -> Result<Vec<(String, String, i64, i64, i64, String)>> {
    let db = self.db.lock().map_err(|e| EflowError::Internal(e.to_string()))?;
    let sql = if include_side {
        "SELECT id, name, created_at, updated_at, message_count, relation
         FROM sessions ORDER BY updated_at DESC LIMIT 50"
    } else {
        "SELECT id, name, created_at, updated_at, message_count, relation
         FROM sessions WHERE relation = 'main' ORDER BY updated_at DESC LIMIT 20"
    };
    let mut stmt = db.prepare(sql).map_err(|e| EflowError::Memory(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| EflowError::Memory(e.to_string()))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| EflowError::Memory(e.to_string()))?);
    }
    Ok(result)
}

/// 保存子会话（subagent 用）
#[allow(clippy::too_many_arguments)]
pub fn save_side_session(
    &self,
    session_id: &str,
    parent_session_id: &str,
    role: &str,
    name: &str,
    messages: &[Message],
) -> Result<()> {
    let db = self.db.lock().map_err(|e| EflowError::Internal(e.to_string()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    db.execute(
        "INSERT INTO sessions
            (id, name, created_at, updated_at, message_count, relation, parent_session_id, role)
         VALUES (?1, ?2, ?3, ?3, ?4, 'side', ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            updated_at = excluded.updated_at,
            message_count = excluded.message_count",
        params![session_id, name, now, messages.len() as i64, parent_session_id, role],
    ).map_err(|e| EflowError::Memory(e.to_string()))?;

    db.execute(
        "DELETE FROM session_messages WHERE session_id = ?1",
        params![session_id],
    ).map_err(|e| EflowError::Memory(e.to_string()))?;

    for msg in messages {
        db.execute(
            "INSERT INTO session_messages (session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, msg.role_str(), msg.content, now],
        ).map_err(|e| EflowError::Memory(e.to_string()))?;
    }
    Ok(())
}
```

- [ ] **Step 2: 修改 `list_sessions()` 包装**

将现有 `list_sessions` 改为：

```rust
pub fn list_sessions(&self) -> Result<Vec<(String, String, i64, i64, i64)>> {
    self.list_sessions_filtered(false)
        .map(|rows| rows.into_iter().map(|(id, name, c, u, n, _rel)| (id, name, c, u, n)).collect())
}
```

- [ ] **Step 3: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-infra/src/memory/session_store.rs && git commit -m "infra(memory): SessionStore 加 list_sessions_filtered + save_side_session

list_sessions() 默认隐藏 side relation（向后兼容）。
save_side_session 给 subagent 持久化子会话用。"
```

---

### Task B2-3: 实现 `SubagentExecutor`（替换占位）

**Files:**
- Modify: `crates/qbird-code-agents/src/subagent/executor.rs` （替换 B1-2 占位为真实实现）
- Create: `crates/qbird-code-agents/src/subagent/executor_test.rs`

- [ ] **Step 1: 替换 `executor.rs` 完整实现**

```rust
//! `SubagentExecutor` — 子 agent 生命周期管理。
//!
//! 给 LLM 派发子任务时调用 `spawn_child_with_provider`：
//! 1. 查 profile（不在则返回 `SubagentProfileNotFound`）
//! 2. 根据 `tool_policy` 构造独立工具集
//! 3. 创建独立 ReactLoop 实例
//! 4. 跑完返回 `ChildRecord`
//!
//! 设计参考 `F:\AI\Kun\kun\src\delegation\child-agent-executor.ts`。
//! v0.3.1 简化：同步等子 agent 完成（detach 留 v0.4）。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::Provider;
use qbird_code_models::{EflowError, Message, UsageStats};
use qbird_code_tools::ToolRegistry;

use crate::react_loop::{ReactLoop, ReactLoopConfig};

use super::profile::{SubagentProfile, ToolPolicy};

#[derive(Debug, Clone, Default)]
pub struct SubagentSpawnHints {
    pub parent_session_id: Option<String>,
    pub parent_turn_id: Option<String>,
    pub detached: bool,
    pub priority: SpawnPriority,
    pub on_event: Option<Arc<dyn Fn(ChildEvent) + Send + Sync>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SpawnPriority {
    #[default]
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub enum ChildEvent {
    Started { child_id: String },
    Completed { summary: String, usage: UsageStats },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct ChildRecord {
    pub child_id: String,
    pub status: ChildStatus,
    pub summary: String,
    pub usage: UsageStats,
    pub profile: String,
    pub tool_policy: ToolPolicy,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Completed,
    Failed,
}

pub struct SubagentExecutor {
    profiles: HashMap<String, SubagentProfile>,
    base_config: ReactLoopConfig,
    tool_registry: Arc<ToolRegistry>,
}

pub struct SubagentExecutorBuilder {
    profiles: Option<HashMap<String, SubagentProfile>>,
    base_config: Option<ReactLoopConfig>,
    tool_registry: Option<Arc<ToolRegistry>>,
}

impl SubagentExecutor {
    pub fn builder() -> SubagentExecutorBuilder {
        SubagentExecutorBuilder {
            profiles: None,
            base_config: None,
            tool_registry: None,
        }
    }

    /// 校验 profile 是否存在
    pub fn validate_profile(&self, name: &str) -> Result<&SubagentProfile, EflowError> {
        self.profiles.get(name).ok_or_else(|| {
            EflowError::SubagentProfileNotFound { name: name.to_string() }
        })
    }

    /// 列出所有 profile 名字
    pub fn list_profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    /// 跑子 agent 到完成（v0.3.1：同步等）
    pub async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        prompt: &str,
        hints: &SubagentSpawnHints,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ChildRecord, EflowError> {
        let profile = self.validate_profile(profile_name)?.clone();
        let child_id = uuid::Uuid::new_v4().to_string();
        let started = Instant::now();

        if let Some(cb) = &hints.on_event {
            cb(ChildEvent::Started { child_id: child_id.clone() });
        }

        let max_iter = profile.max_iterations.unwrap_or(self.base_config.max_iterations);
        let child_config = ReactLoopConfig {
            max_iterations: max_iter,
            model: profile.model.clone()
                .unwrap_or_else(|| self.base_config.model.clone()),
            ..self.base_config.clone()
        };

        let system_prompt = format!(
            "{}\n\n[父代理任务]\n{}\n\n[约束]\n- 你是子代理，独立完成任务\n- 完成后简洁汇报结果",
            profile.prompt_preamble, prompt
        );
        let mut messages = vec![
            Message::system(&system_prompt),
            Message::user(prompt),
        ];

        let child_tool_schemas = match profile.tool_policy {
            ToolPolicy::ReadOnly => self.read_only_tool_schemas(),
            ToolPolicy::Inherit => self.base_tool_schemas(),
        };

        let react_loop = ReactLoop::new(child_config);
        let result = react_loop
            .run(
                provider,
                http_client,
                &mut messages,
                &child_tool_schemas,
                &self.tool_registry,
                Some(max_iter),
                None,
                None,
            )
            .await;

        let duration_ms = started.elapsed().as_millis() as u64;

        match result {
            Ok(agent_result) => {
                let record = ChildRecord {
                    child_id,
                    status: ChildStatus::Completed,
                    summary: agent_result.content,
                    usage: agent_result.usage,
                    profile: profile_name.to_string(),
                    tool_policy: profile.tool_policy,
                    duration_ms,
                };
                if let Some(cb) = &hints.on_event {
                    cb(ChildEvent::Completed {
                        summary: record.summary.clone(),
                        usage: record.usage.clone(),
                    });
                }
                Ok(record)
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                if let Some(cb) = &hints.on_event {
                    cb(ChildEvent::Failed { error: err_msg.clone() });
                }
                Ok(ChildRecord {
                    child_id,
                    status: ChildStatus::Failed,
                    summary: err_msg,
                    usage: UsageStats::default(),
                    profile: profile_name.to_string(),
                    tool_policy: profile.tool_policy,
                    duration_ms,
                })
            }
        }
    }

    fn read_only_tool_schemas(&self) -> Vec<serde_json::Value> {
        let read_only = SubagentProfile::read_only_tool_names();
        self.tool_registry
            .definitions()
            .into_iter()
            .filter(|d| read_only.contains(&d.name.as_str()))
            .map(|d| serde_json::json!({
                "type": "function",
                "function": {
                    "name": d.name,
                    "description": d.description,
                    "parameters": d.parameters,
                }
            }))
            .collect()
    }

    fn base_tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tool_registry
            .definitions()
            .into_iter()
            .map(|d| serde_json::json!({
                "type": "function",
                "function": {
                    "name": d.name,
                    "description": d.description,
                    "parameters": d.parameters,
                }
            }))
            .collect()
    }
}

impl SubagentExecutorBuilder {
    pub fn profiles(mut self, profiles: HashMap<String, SubagentProfile>) -> Self {
        self.profiles = Some(profiles);
        self
    }
    pub fn base_config(mut self, config: ReactLoopConfig) -> Self {
        self.base_config = Some(config);
        self
    }
    pub fn tool_registry(mut self, registry: Arc<ToolRegistry>) -> Self {
        self.tool_registry = Some(registry);
        self
    }
    pub fn build(self) -> Result<SubagentExecutor, EflowError> {
        Ok(SubagentExecutor {
            profiles: self.profiles.ok_or_else(||
                EflowError::Internal("SubagentExecutor: profiles 必填".into()))?,
            base_config: self.base_config.ok_or_else(||
                EflowError::Internal("SubagentExecutor: base_config 必填".into()))?,
            tool_registry: self.tool_registry.ok_or_else(||
                EflowError::Internal("SubagentExecutor: tool_registry 必填".into()))?,
        })
    }
}
```

- [ ] **Step 2: 创建 `executor_test.rs`**

```rust
use std::collections::HashMap;
use std::sync::Arc;

use qbird_code_models::EflowError;
use qbird_code_tools::ToolRegistry;

use crate::react_loop::ReactLoopConfig;
use crate::subagent::executor::SubagentExecutor;
use crate::subagent::profile::{SubagentMode, SubagentProfile, ToolPolicy};

fn make_profiles() -> HashMap<String, SubagentProfile> {
    let mut m = HashMap::new();
    m.insert(
        "explore".into(),
        SubagentProfile {
            name: "explore".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: "你是探索代理".into(),
            description: "test".into(),
            default_tools: vec!["read_file".into(), "search_code".into()],
            max_iterations: Some(5),
            model: None,
        },
    );
    m
}

#[test]
fn executor_builder_succeeds_with_valid_inputs() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build();
    assert!(executor.is_ok());
}

#[test]
fn executor_validate_profile_returns_not_found_for_unknown() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build()
        .unwrap();
    let result = executor.validate_profile("nonexistent");
    assert!(matches!(result, Err(EflowError::SubagentProfileNotFound { .. })));
}

#[test]
fn executor_list_profile_names_returns_all_loaded() {
    let profiles = make_profiles();
    let registry = Arc::new(ToolRegistry::new());
    let executor = SubagentExecutor::builder()
        .profiles(profiles)
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build()
        .unwrap();
    let names = executor.list_profile_names();
    assert!(names.contains(&"explore".to_string()));
}

#[test]
fn executor_builder_missing_profiles_errors() {
    let registry = Arc::new(ToolRegistry::new());
    let result = SubagentExecutor::builder()
        .base_config(ReactLoopConfig::default())
        .tool_registry(registry)
        .build();
    assert!(result.is_err());
}
```

- [ ] **Step 3: 跑测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-agents subagent::executor
```

Expected: 4 个测试 PASS

- [ ] **Step 4: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-agents/src/subagent/executor.rs crates/qbird-code-agents/src/subagent/executor_test.rs && git commit -m "agents(subagent): 实现 SubagentExecutor 真实子 agent 跑逻辑

- Builder 模式构造
- validate_profile 查表
- spawn_child_with_provider 跑独立 ReactLoop
- tool_policy=ReadOnly 过滤工具集
- 4 个单元测试覆盖 builder / validate / list / missing fields"
```

---

### Task B2-4: 实现 `DelegateTaskTool`（LLM 主动调用入口）

**Files:**
- Create: `crates/qbird-code-tools/src/delegate_task.rs`
- Create: `crates/qbird-code-tools/src/delegate_task_test.rs`
- Modify: `crates/qbird-code-tools/src/lib.rs`

- [ ] **Step 1: 创建 `delegate_task.rs`**

```rust
//! `DelegateTaskTool` — 让 LLM 主动派发子任务给 subagent。
//!
//! 设计参考 `F:\AI\Kun\kun\src\adapters\tool\delegation-tool-provider.ts`。
//! v0.3.1 简化：同步等子 agent 完成；detach 留 v0.4。

use std::sync::Arc;

use async_trait::async_trait;
use qbird_code_agents::subagent::{ChildRecord, SubagentExecutor};
use qbird_code_infra::http_client::HttpLlmClient;
use qbird_code_infra::providers::Provider;
use qbird_code_models::{EflowError, Result};
use rust_i18n::t;
use serde_json::json;

use crate::registry::{Tool, ToolDefinition, ToolOutput};

/// 抽象：让 DelegateTaskTool 不直接依赖具体 SubagentExecutor（方便测试 mock）
#[async_trait]
pub trait SubagentExecutorTrait: Send + Sync {
    fn list_profile_names(&self) -> Vec<String>;
    fn validate_profile(&self, name: &str) -> Result<(), EflowError>;
    async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        prompt: &str,
        hints: &qbird_code_agents::subagent::SubagentSpawnHints,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ChildRecord, EflowError>;
}

#[async_trait]
impl SubagentExecutorTrait for SubagentExecutor {
    fn list_profile_names(&self) -> Vec<String> {
        SubagentExecutor::list_profile_names(self)
    }
    fn validate_profile(&self, name: &str) -> Result<(), EflowError> {
        SubagentExecutor::validate_profile(self, name).map(|_| ())
    }
    async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        prompt: &str,
        hints: &qbird_code_agents::subagent::SubagentSpawnHints,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ChildRecord, EflowError> {
        SubagentExecutor::spawn_child_with_provider(
            self, profile_name, prompt, hints, provider, http_client
        ).await
    }
}

pub struct DelegateTaskTool {
    executor: Arc<dyn SubagentExecutorTrait>,
}

impl DelegateTaskTool {
    /// 生产入口：接受 trait object（生产用 `Arc<SubagentExecutor>`，测试用 mock）
    pub fn new(executor: Arc<dyn SubagentExecutorTrait>) -> Self {
        Self { executor }
    }

    /// 暴露 executor 引用（测试用）
    pub fn executor(&self) -> &Arc<dyn SubagentExecutorTrait> {
        &self.executor
    }

    /// 真实执行（带 provider / http_client）
    pub async fn execute_with_provider(
        &self,
        params: serde_json::Value,
        provider: &dyn Provider,
        http_client: &HttpLlmClient,
    ) -> Result<ToolOutput> {
        let label = params.get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EflowError::Internal(
                t!("err_tool_missing_param", name = "label").to_string()
            ))?;
        let prompt = params.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EflowError::Internal(
                t!("err_tool_missing_param", name = "prompt").to_string()
            ))?;
        let profile_name = params.get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        self.executor.validate_profile(profile_name)?;

        let hints = qbird_code_agents::subagent::SubagentSpawnHints::default();
        let record = self.executor.spawn_child_with_provider(
            profile_name, prompt, &hints, provider, http_client
        ).await?;

        let output_json = json!({
            "child_id": record.child_id,
            "label": label,
            "status": format!("{:?}", record.status),
            "summary": record.summary,
            "profile": record.profile,
            "tool_policy": format!("{:?}", record.tool_policy),
            "duration_ms": record.duration_ms,
        });

        Ok(ToolOutput {
            success: record.status == qbird_code_agents::subagent::ChildStatus::Completed,
            content: serde_json::to_string_pretty(&output_json).unwrap_or_default(),
            metadata: Some(output_json),
        })
    }
}

#[async_trait]
impl Tool for DelegateTaskTool {
    fn definition(&self) -> ToolDefinition {
        let profiles = self.executor.list_profile_names();
        let profile_names_str = profiles.join(", ");
        ToolDefinition {
            name: "delegate_task".to_string(),
            description: format!(
                "{}\n\n可用 profiles: {}。profile 省略时默认 'general'。",
                t!("tool_delegate_task_description").to_string(),
                profile_names_str
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "label": {"type": "string", "description": "2-4 词子任务标题，UI 显示"},
                    "prompt": {"type": "string", "description": "交给子代理的具体任务"},
                    "workspace": {"type": "string", "description": "子代理工作目录（可选）"},
                    "model": {"type": "string", "description": "覆盖子代理模型（可选，v0.3.1 暂未实现）"},
                    "profile": {
                        "type": "string",
                        "enum": profiles,
                        "description": "子代理角色（默认 general）"
                    }
                },
                "required": ["prompt", "label"],
                "additionalProperties": false
            }),
            risk_level: qbird_code_models::RiskLevel::L2,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolOutput> {
        Err(EflowError::Internal(
            "DelegateTaskTool 必须通过 execute_with_provider 调用".into()
        ))
    }
}
```

- [ ] **Step 2: 加 i18n key**

`locales/zh-CN.yml` 加：

```yaml
tool_delegate_task_description: "派发子任务给 subagent 独立执行。可同时发多个 delegate_task 实现并行；超并行预算时排队。子 agent 完成后返回 summary。"
```

`locales/en-US.yml` 加：

```yaml
tool_delegate_task_description: "Dispatch a sub-task to a subagent for isolated execution. Issue several delegate_task calls in one message to investigate in parallel; runs queue when parallel budget is full. Returns the child agent's summary on completion."
```

- [ ] **Step 3: 在 `tools/src/lib.rs` 加 pub use**

```rust
pub mod delegate_task;
pub use delegate_task::DelegateTaskTool;
```

- [ ] **Step 4: 创建 `delegate_task_test.rs`**

```rust
use std::sync::Arc;
use async_trait::async_trait;
use qbird_code_agents::subagent::{
    ChildRecord, ChildStatus, SubagentExecutor, SubagentSpawnHints, SubagentExecutorTrait, ToolPolicy,
};
use qbird_code_models::{EflowError, Result};
use qbird_code_tools::delegate_task::DelegateTaskTool;
use qbird_code_tools::Tool;
use serde_json::json;

struct MockExecutor;

#[async_trait]
impl SubagentExecutorTrait for MockExecutor {
    fn list_profile_names(&self) -> Vec<String> {
        vec!["general".into(), "explore".into()]
    }
    fn validate_profile(&self, name: &str) -> Result<(), EflowError> {
        if name == "general" || name == "explore" {
            Ok(())
        } else {
            Err(EflowError::SubagentProfileNotFound { name: name.into() })
        }
    }
    async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        _prompt: &str,
        _hints: &SubagentSpawnHints,
        _provider: &dyn qbird_code_infra::providers::Provider,
        _http: &qbird_code_infra::http_client::HttpLlmClient,
    ) -> Result<ChildRecord, EflowError> {
        Ok(ChildRecord {
            child_id: "test-child-id".into(),
            status: ChildStatus::Completed,
            summary: format!("mock done for {}", profile_name),
            usage: Default::default(),
            profile: profile_name.into(),
            tool_policy: ToolPolicy::Inherit,
            duration_ms: 100,
        })
    }
}

#[tokio::test]
async fn delegate_task_tool_definition_has_correct_schema() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let def = tool.definition();
    assert_eq!(def.name, "delegate_task");
    assert!(def.description.contains("Available profiles")
         || def.description.contains("可用 profiles"));
    let props = def.parameters["properties"].as_object().expect("properties");
    assert!(props.contains_key("label"));
    assert!(props.contains_key("prompt"));
    assert!(props.contains_key("profile"));
    assert!(props.contains_key("workspace"));
    assert!(props.contains_key("model"));
}

#[tokio::test]
async fn delegate_task_tool_plain_execute_returns_error() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let result = tool.execute(json!({"label": "test"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn delegate_task_tool_unknown_profile_returns_error() {
    let tool = DelegateTaskTool::new(Arc::new(MockExecutor));
    let result = tool.executor().validate_profile("nonexistent");
    assert!(matches!(result, Err(EflowError::SubagentProfileNotFound { .. })));
}
```

- [ ] **Step 5: 跑测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-tools delegate_task
```

Expected: 3 个测试 PASS

- [ ] **Step 6: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 7: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-tools/src/delegate_task.rs crates/qbird-code-tools/src/delegate_task_test.rs crates/qbird-code-tools/src/lib.rs locales/zh-CN.yml locales/en-US.yml && git commit -m "tools: 新建 DelegateTaskTool

LLM 主动调用的 delegate_task 工具入口；抽象 SubagentExecutorTrait
让 mock 测试可行。3 个单元测试覆盖 schema / 普通 execute 报错 / 未知 profile。"
```

---

### Task B2-5: ReactLoop 集成 delegate_task + main.rs 接线

**Files:**
- Modify: `crates/qbird-code-agents/src/react_loop/types.rs` （`ReactLoopConfig` 加 `subagent_executor`）
- Modify: `crates/qbird-code-agents/src/react_loop/mod.rs` （`execute_tools_*` 加 match）
- Modify: `crates/qbird-code/src/main.rs` （注册 DelegateTaskTool + 传入 subagent_executor）

- [ ] **Step 1: 改 `ReactLoopConfig`**

在 `crates/qbird-code-agents/src/react_loop/types.rs` 的 `ReactLoopConfig` struct 末尾加字段：

```rust
/// v0.3.1：可选 subagent executor；delegate_task 工具的真正执行者
pub subagent_executor: Option<std::sync::Arc<crate::subagent::SubagentExecutor>>,
```

加 `Default::default()` 中的初始化：

```rust
subagent_executor: None,
```

- [ ] **Step 2: 改 `execute_tools_sequential` / `execute_tools_parallel`**

读 `crates/qbird-code-agents/src/react_loop/mod.rs:344-377` 的两个 execute 方法。改它们的签名，加 `provider: &dyn Provider, http_client: &HttpLlmClient` 参数。

在两个方法中加 match 分支：

```rust
let content = if tc.function.name == "delegate_task" {
    self.execute_delegate_task(tc, provider, http_client).await?
} else {
    let result = tool_registry
        .execute(&tc.function.name, args, task_id)
        .await?;
    result.content
};
```

加方法到 `impl ReactLoop`：

```rust
async fn execute_delegate_task(
    &self,
    tc: &ToolCall,
    provider: &dyn Provider,
    http_client: &HttpLlmClient,
) -> Result<String, EflowError> {
    let executor = self.config.subagent_executor.as_ref().ok_or_else(|| {
        EflowError::Internal("delegate_task called but subagent_executor is None".into())
    })?;

    let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
        .unwrap_or(serde_json::json!({}));

    let prompt = args.get("prompt").and_then(|v| v.as_str())
        .ok_or_else(|| EflowError::Internal("delegate_task: missing prompt".into()))?;
    let label = args.get("label").and_then(|v| v.as_str())
        .ok_or_else(|| EflowError::Internal("delegate_task: missing label".into()))?;
    let profile = args.get("profile").and_then(|v| v.as_str())
        .unwrap_or("general");

    let hints = crate::subagent::SubagentSpawnHints::default();
    let record = executor.spawn_child_with_provider(
        profile, prompt, &hints, provider, http_client
    ).await?;

    let output = serde_json::json!({
        "child_id": record.child_id,
        "label": label,
        "status": format!("{:?}", record.status),
        "summary": record.summary,
        "profile": record.profile,
        "tool_policy": format!("{:?}", record.tool_policy),
        "duration_ms": record.duration_ms,
    });

    Ok(serde_json::to_string_pretty(&output).unwrap_or_default())
}
```

- [ ] **Step 3: main.rs 添加 imports + 构造 SubagentExecutor + 注册 DelegateTaskTool**

在 `crates/qbird-code/src/main.rs` 顶部 imports 加：

```rust
use qbird_code_agents::subagent::load_profiles;
use qbird_code_tools::DelegateTaskTool;
```

在 EditTool 注册后（约 main.rs:368 之后），构造并注册：

```rust
// === 4c. 构造 SubagentExecutor + 注册 DelegateTaskTool ===
// v0.3.1 简化路径：只用内置 5 个 profile
let subagent_profiles = load_profiles(None)
    .map_err(|e| {
        eprintln!("{}", e.user_message());
        e
    })?;

// 构造 SubagentExecutor；用临时 Arc<ToolRegistry>（后面会再 wrap）
let temp_registry_arc = std::sync::Arc::new(registry.clone());
let subagent_executor = std::sync::Arc::new(
    qbird_code_agents::subagent::SubagentExecutor::builder()
        .profiles(subagent_profiles)
        .base_config(ReactLoopConfig {
            model: model.clone(),
            ..ReactLoopConfig::default()
        })
        .tool_registry(temp_registry_arc)
        .build()
        .map_err(|e| {
            eprintln!("{}", e.user_message());
            e
        })?
);
let delegate_task_tool = DelegateTaskTool::new(
    subagent_executor.clone() as std::sync::Arc<dyn qbird_code_tools::delegate_task::SubagentExecutorTrait>
);
```

调整 `let tool_registry = Arc::new(registry);` 为：

```rust
registry.register(std::sync::Arc::new(delegate_task_tool));
let tool_registry = std::sync::Arc::new(registry);
```

- [ ] **Step 4: main.rs 2 处 ReactLoop 构造都传入 subagent_executor**

main.rs 现有 2 处构造 `ReactLoop::new(ReactLoopConfig { ... })`（execute 模式 + interactive 模式），都加：

```rust
subagent_executor: Some(subagent_executor.clone()),
```

- [ ] **Step 5: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 6: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-agents/src/react_loop/types.rs crates/qbird-code-agents/src/react_loop/mod.rs crates/qbird-code/src/main.rs && git commit -m "agents(react-loop) + cli: 集成 delegate_task 工具

- ReactLoopConfig 加 subagent_executor 字段
- execute_tools_* 加 match 分支：delegate_task 直接走 SubagentExecutor
- main.rs 构造 SubagentExecutor + 注册 DelegateTaskTool
- 2 处 ReactLoop 构造都传入 subagent_executor"
```

---

### Task B2-6: 集成测试

**Files:**
- Create: `crates/qbird-code-tools/tests/delegate_task_integration_test.rs`

- [ ] **Step 1: 创建集成测试**

```rust
//! 集成测试：验证 delegate_task 工具注册后，definitions 中能看到它
//! 用真实 ToolRegistry + 真实 DelegateTaskTool（mock executor）

use std::sync::Arc;
use async_trait::async_trait;
use qbird_code_models::EflowError;
use qbird_code_tools::delegate_task::{DelegateTaskTool, SubagentExecutorTrait};
use qbird_code_agents::subagent::{ChildRecord, ChildStatus, SubagentSpawnHints, ToolPolicy};
use qbird_code_tools::Tool;
use qbird_code_tools::ToolRegistry;

struct MockExecutor;

#[async_trait]
impl SubagentExecutorTrait for MockExecutor {
    fn list_profile_names(&self) -> Vec<String> {
        vec!["general".into(), "explore".into()]
    }
    fn validate_profile(&self, name: &str) -> Result<(), EflowError> {
        if name == "general" || name == "explore" { Ok(()) }
        else { Err(EflowError::SubagentProfileNotFound { name: name.into() }) }
    }
    async fn spawn_child_with_provider(
        &self,
        profile_name: &str,
        _prompt: &str,
        _hints: &SubagentSpawnHints,
        _provider: &dyn qbird_code_infra::providers::Provider,
        _http: &qbird_code_infra::http_client::HttpLlmClient,
    ) -> Result<ChildRecord, EflowError> {
        Ok(ChildRecord {
            child_id: "mock-child".into(),
            status: ChildStatus::Completed,
            summary: format!("mock result for profile={}", profile_name),
            usage: Default::default(),
            profile: profile_name.into(),
            tool_policy: ToolPolicy::Inherit,
            duration_ms: 42,
        })
    }
}

#[test]
fn delegate_task_tool_appears_in_registry_definitions() {
    let mut registry = ToolRegistry::new();
    let mock: Arc<dyn SubagentExecutorTrait> = Arc::new(MockExecutor);
    let tool = DelegateTaskTool::new(mock);
    registry.register(Arc::new(tool));

    let defs = registry.definitions();
    let delegate_def = defs.iter().find(|d| d.name == "delegate_task");
    assert!(delegate_def.is_some(), "delegate_task must be in registry");
    let def = delegate_def.unwrap();
    assert!(def.description.contains("general"));
    assert!(def.description.contains("explore"));
}
```

- [ ] **Step 2: 跑测试**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo test -p qbird-code-tools --test delegate_task_integration_test
```

Expected: PASS

- [ ] **Step 3: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add crates/qbird-code-tools/tests/delegate_task_integration_test.rs && git commit -m "tools: 集成测试 delegate_task 注册到 ToolRegistry"
```

---

### Task B2-7: 文档 + CHANGELOG + CLAUDE.md 同步

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `CLAUDE.md`
- Modify: `docs/cli.md`

- [ ] **Step 1: CHANGELOG.md 追加**

在 v0.3.1 段加：

```markdown
### Added
- **Subagent 系统完整实现**：`SubagentProfile` + 5 个内置 profile
  + `SubagentExecutor` + `delegate_task` 工具 + SessionStore 子会话持久化
  （`relation: 'side'` + `parent_session_id` + `role` 字段）。
- **delegate_task 工具**：LLM 可主动派发子任务给独立 ReAct 循环实例。
- **SessionStore schema 迁移**：sessions 表加 `relation` / `parent_session_id`
  / `role` 3 个字段；老库自动 ALTER 迁移。
- **预留 forward-compat 钩子**：`SubagentSpawnHints`（含 detached / priority /
  on_event）供 v0.4 进化系统（CompactionManager / Reflection Engine / Profile
  Compilation）接入。

### Migration Notes
- 现有 v0.3.0 SessionStore 数据库自动迁移（ALTER TABLE）；用户无感。
- v0.3.0 中已弃用的 `Subagent` struct / `execute_parallel` 已彻底删除。
```

- [ ] **Step 2: CLAUDE.md 状态表更新**

顶部"当前状态"表更新：

```markdown
| **当前版本** | 0.3.1 |
| **已完成** | v0.3.0 + v0.3.1（agent 身份 + subagent 完整重写） |
| **下一步** | v0.4.0 进化系统：CompactionManager / MemoryZone / Reflection Engine |
```

加 v0.3.1 子段。

- [ ] **Step 3: docs/cli.md 加 subagent 章节**

```markdown
## Subagent 系统

qingbird 在 v0.3.1 引入了 subagent 机制：主 agent 可以通过
`delegate_task` 工具派发子任务给独立的 ReAct 循环实例。

### 5 个内置 profile

| Profile | 工具策略 | 适用场景 |
|---|---|---|
| `general` | 继承主 agent | 多步任务、读写文件、跑命令 |
| `explore` | 只读 | 快速查找文件、搜索代码 |
| `code-writer` | 继承 | 实现功能、修 bug |
| `planner` | 只读 | 推理设计实施方案 |
| `reviewer` | 只读 | 代码审查、报告问题 |

### LLM 何时调用

LLM 在判断任务复杂、需要独立上下文、或适合并行时主动调用：

```json
{
  "label": "审查登录流程",
  "prompt": "阅读 src/auth/login.rs 并报告 3 个最严重的代码问题",
  "profile": "reviewer"
}
```

一次发多个 `delegate_task` 实现并行执行。

### 子会话持久化

子 agent 的完整历史保存到 SessionStore，标记 `relation: 'side'` 和
`parent_session_id` 指向主会话。`/sessions` 默认隐藏，
未来 `/sessions --include-side` 可列出。
```

- [ ] **Step 4: 跑 4 门禁**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build
```

Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && git add CHANGELOG.md CLAUDE.md docs/cli.md && git commit -m "docs: CHANGELOG v0.3.1 段补全 + CLAUDE.md 状态同步 + cli.md subagent 章节"
```

---

### Task B2-8: 最终验证（端到端手测 + 全部 4 门禁）

- [ ] **Step 1: 端到端手测 execute 模式**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo build --release
cat > /tmp/test-qingbird.yaml <<EOF
llm:
  active: deepseek
  deepseek:
    api_key: dummy
    default_model: deepseek-v4-pro
core:
  language: zh-CN
security:
  risk_threshold: L3
  allowed_paths:
    - "."
EOF

RUST_LOG=info ./target/release/qingbird --execute \
  --config /tmp/test-qingbird.yaml \
  "你好"
```

Expected: 输出简单的中文问候（**不**问"你想做什么代码"）。

- [ ] **Step 2: 端到端手测 interactive 模式**

```bash
RUST_LOG=info ./target/release/qingbird --interactive \
  --config /tmp/test-qingbird.yaml
```

输入 `你好` → 期望中文问候。
输入 `请派一个子代理去搜 xxx` → 期望 LLM 调 `delegate_task`。
输入 `/quit` 退出。

- [ ] **Step 3: 4 门禁全过**

```bash
cd "F:\AI data\Claude Code\qingbird-code" && cargo fmt --check && cargo clippy --all-targets --workspace -- -D warnings && cargo test --workspace && cargo build --release
```

Expected: 全部通过

- [ ] **Step 4: 验证完成**

无代码改动；plan 完成。

---

## Out of Scope（v0.4+ 路线，不在本 plan）

来自 `F:\AI\agent-evolution-with-hermes.md` 的 4 阶段路线，本 plan **不**实现，但**预留了 forward-compat 钩子**：

- **v0.4.0 阶段 1（1-2 周）**
  - `CompactionManager` — LLM 摘要压缩。**用 `SubagentExecutor` 跑压缩任务**（`profile: "compactor"`）。
  - `MemoryZone` — `memory/types.rs` 加 `Core/Work/Project/Episode/General` 枚举。**复用 SessionStore `role` 字段做 zone 标签**。
  - 消除 ContextManager 双份数据。
- **v0.4.0 阶段 2（2-3 周）**
  - `Profile Compilation` — `/compile` 命令。**用 `SubagentExecutor` 跑编译**。
  - `MetricsCollector` + `/metrics` 命令。
  - `FeedbackCollector`。
  - `Supersedes` 版本链。
- **v0.4.0 阶段 3（3-4 周）**
  - 微反思（每 3 轮 async）— **用 `SubagentExecutor::spawn_child` 加 `SpawnPriority::Low` + `on_event` 回调**实现。
  - 全量反思 `/reflect`。
  - 效果追踪 (Loaded/Referenced)。
  - `StrategyStore`。
  - `ParameterTuner`。
- **v0.4.0 阶段 4（4-6 周）**
  - 审计日志。
  - 经验 → 技能自动晋升。
  - `ToolSuggester`。
  - 跨会话持续学习闭环。

**关键对接点**（已在本 plan 预留）：

| 进化组件 | 用 SubagentExecutor 的方式 |
|---|---|
| CompactionManager | `executor.spawn_child_with_provider("compactor", &summarize_prompt, &hints, ...)` |
| Reflection Engine | `hints.priority = SpawnPriority::Low`, `hints.on_event = Some(metrics_cb)` |
| Profile Compilation | `executor.spawn_child_with_provider("compiler", &compile_prompt, ...)` |
| 子 agent metrics 收集 | `hints.on_event` 回调接 MetricsCollector |

---

## 已知风险 + 缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| SessionStore schema 迁移在某些边缘情况失败 | 低 | 中 | 迁移逻辑有列存在检查；B2-1 验证 `cargo test` 跑现有测试 |
| `delegate_task` 工具 LLM 不知道何时调用 | 中 | 中 | system_prompt 加 tool behavior 段；profile description 在 tool description 中列出 |
| ReactLoop 集成 delegate_task 的 match 分支不易扩展 | 中 | 低 | 当前只 match `delegate_task`；v0.4 加新工具时考虑引入 `ToolExecutionContext` 抽象 |
| yaml subagent 扩展不在本 PR | 中 | 低 | v0.3.1 只用内置 5 个 profile；yaml 加载逻辑在 PR-B1 已写好（`load_profiles_from_yaml`），v0.3.2 接 config schema 即可 |
| LLM 反复派 subagent 烧 token | 中 | 中 | `max_iterations` 由 profile 限制（explore=15, planner=10, reviewer=12）；v0.4 加 budget tracking |
| detach 模式未实现 | 低 | 低 | v0.3.1 同步等结果；detach 留 v0.4（`SubagentSpawnHints.detached` 字段已预留） |

---

## 剩余假设

1. **5 个内置 profile 命名/描述合理** — 假设用户觉得 "general / explore / code-writer / planner / reviewer" 这套命名直观。yaml 用户扩展支持自定义（v0.3.2+）。
2. **DeepSeek 模型能理解 delegate_task 工具调用** — 假设是。如果不行，需要换更聪明的模型或简化 tool description。
3. **Provider trait 设计支持 v0.4 reflection 的 small model 路由** — 假设是。`SubagentProfile.model` 字段已预留。
4. **SessionStore `relation` 字段加 3 个列不影响查询性能** — 假设是（SQLite 列数对性能无明显影响）。
5. **删除 `Subagent` / `execute_parallel` 无破坏性** — 已 grep 验证无外部用户。
