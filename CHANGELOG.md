# Changelog

All notable changes to eflow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added (TUI 交互)

- **TUI 交互层** (F1-F6): ratatui + crossterm 异步终端 UI，事件流实时刷新，prompt 异步输入（设计 §14.3）
- **InteractionLayer trait** (F2): 核心层与交互层解耦，未来 GUI / Web 接入零侵入
- **TUI 默认启动** (F6): 无参数启动进入 TUI；`--execute` / `--show-config` / `--list-profiles` 保留 CLI 直输出

### Added (并行派发)

- **Orchestrator 真正并行派发** (E4): `compute_step_layers` 把 TaskPlan 按依赖分层，每层内步骤用 `FuturesUnordered` 并发执行（设计 §5.1）
- **SubagentHandle 暴露 pool active map guard** (E1): 调度方能拿到 agent 角色 / 能力做决策
- **SubagentPool.list_active** (E2): 暴露活跃 agent 元数据给 Orchestrator
- **SubagentPool timeout-based cleanup_idle** (E5): 5min 默认 idle timeout（v1.1 行为兼容），超时未活动的 agent 自动回收（设计 §13.3）
- **LlmRouter 测试 helper** (E6): `placeholder` / `inject_test_provider` / `inject_test_routing` — `#[doc(hidden)]` 标"非测试代码不应调用"

### Changed

- **Subagent 加 `created_at` 字段** (E5): 用于 idle cleanup 计算
- **Parallel execution 集成测试** (E6): `tests/parallel_execution_test.rs` 验证 E4 分层派发 + D→E→F 集成不破

### Changed (P1 债务清理)

- **L2 cache key 规范化** (D1): `decisioner`/`executor`/`feedbacker` 三处 CacheKey 构造抽到 `cache_key_for_step` helper；signature 只含 action+tool+intent，不含 params（params 变化不再破 cache 命中）
- **Feedbacker cache key 行为变化** (D1): 之前 `feedback:retry={N}:op={summary}` 每次 retry 都换 key；改为 `{intent}:{tool}:{summary}`，retry 期间稳定 → 命中率提升
- **Concierge 真接入 active_profile** (D3): `ProfileSwitch` 意图从"只发提示"升级为真切换，`concierge.active_profile()` getter 可读
- **Concierge 派发前 recall 历史** (D4): `TaskDispatch` 派发前 recall 项目记忆（关键词取 description 前 32 字符）

### Planned

- Additional LLM providers (per design doc v4.0 §10)

---

## [1.1.0] - 2026-06-16

### Added

- **M4.5 LLM 硬化** (设计 §11.2): Config-driven timeout/retry、exponential backoff、L1 prefix cache 接线、模型 tier 降级路径
- **M8 L2 结构化缓存**: 内存 LRU + SQLite 磁盘，命中率监控（`MemoryLruBackend` + `SqliteCacheBackend` + `L2CacheManager` + `LlmRouter::chat_cached`）
- **M10.5 多 Subagent 并发池** (设计 §13.3): `SubagentPool` mpsc + N worker + `SubagentHandle` RAII drop 归还 + role-based capability 路由 + `cleanup_idle` 占位 + `Orchestrator::with_pool` 接入

### Changed (破坏性 / 向后兼容)

- `ProviderEntry` 新增 `timeout_secs` / `max_retries` / `retry_backoff_ms` 字段（带默认值，向后兼容）
- `CacheConfig` 新增 `l2_enabled` / `l2_ttl_days` 字段
- `LlmRouter` 新增 `l2_cache` 字段，`from_config` 签名不变（内部读取 cache 配置）
- `Subagent::new` 根据 capabilities 推导 `PermissionSet`（ExecuteCommand 解锁命令白名单等）

### Fixed

- QA B2: LLM Provider timeout 接线（关硬编码）

## [1.0.0] - 2026-06-15

### Added

**Initial release of eflow — end-to-end runnable skeleton.**

#### Architecture (4 layers)
- **Interaction layer**: clap 4 derive CLI (`src/interaction/{mod,cli}.rs`)
- **Orchestration layer**: zero-blocking Concierge + task-decomposing Orchestrator (`src/application/`)
- **Capability layer**: D→E→F pipeline (Decisioner → Executor → Feedbacker) + Subagent with retry loop
- **Infrastructure layer**: LLM Router / 3-layer Memory / Context compressor / Event bus / Profile / Tools / Locale

#### Features
- **Zero-blocking dialogue**: Concierge dispatches tasks via `tokio::spawn`; main thread never waits
- **Multi-LLM provider**: Anthropic Claude + OpenAI; routed by `ModelTier` (Strong/Medium/Light)
- **Three-layer memory**: WorkingMemory (LRU in-memory) + ProjectMemory (SQLite FTS5) + UserMemory
- **D→E→F pipeline** with feedback loop and risk escalation (RiskLevel L0–L3)
- **Event bus**: `tokio::broadcast` with 6 event types (TaskStarted / TaskCompleted / TaskFailed / RiskEscalated / UserInputRequired / SystemShutdown)
- **i18n bilingual**: zh-CN (default) + en-US via `rust-i18n` compile-time macro; 80+ i18n keys
- **Atomic tools**: `read_file` / `write_file` / `execute_command` / `search_code` (Rust-native regex via `regex-lite`)
- **Profile + Skill system**: industry-profile-driven system prompts + skill templates; YAML loaded
- **Context compression**: L1 structural (action log → summary; file → ref pointer) + L2 semantic
- **Cross-platform**: Windows / Linux / macOS (Windows compatibility prioritized)

#### Testing
- 12 test files covering all major modules
- 11 end-to-end integration tests (`tests/integration_test.rs`)
- LLM-required paths use dummy API keys + 5s timeout to prevent hangs

#### Documentation
- Architecture design document v4.0
- Implementation plan for v1.0 milestones M0–M14
- `CLAUDE.md` with strict development rules and handoff protocol
- `WORKLOG.md` complete change archive

### Notes

- OpenAI streaming is a stub in v1.0 (returns "not yet implemented" error)
- LLM `Client::timeout()` is not yet wired to `cfg.llm.timeout_secs` — tracked for v1.1

---

## [1.3.0] (TBD)

**Breaking Changes**:

- **`eflow.yaml::llm.providers` 字段删除** —— v1.2 的 `anthropic` / `openai` 硬编码字段不再存在。LLM provider 改用 `~/.eflow/providers/{name}.yaml` 独立文件管理。详见 `docs/migration-v1.2-to-v1.3.md`。

**Features**:

- **核心 crate 零预置 LLM provider** —— 4 家 preset 厂商（DeepSeek / MiniMax / agnes-ai / OpenCode Go）以示例 YAML 形式存在，不进 core
- **Generic OpenAI 兼容 / Anthropic 兼容 adapter** —— 通过 `~/.eflow/providers/*.yaml` 加载
- **env var 退化路径** —— `~/.eflow/providers/` 为空时回退到 v1.2 的 `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` 行为

**Internal**:

- ADR-0011 核心零预置 provider
- ADR-0012 配置格式破坏性变更
- ADR-0013 GUI 接入路径从 InteractionLayer trait 改为 CLI 稳定契约（spec B 实施）

---

## [1.3.1] (TBD)

**Features**:

- **6 个斜杠命令**：`/model` / `/profile` / `/lang` / `/level`（空壳，spec C 实施）/ `/help` / `/quit`
- **首次配置向导**：`eflow init` 强制进；首次启动检测无配置时提示是否进
- **SelectList widget**：多模交互（输入序号 / ↑↓ 键 / 鼠标滚轮 / 鼠标点击 / Enter / Esc）
- **bare TUI 模式**：未配置时启动仍能看界面 + header 显 "⚠ 未配置"
- **核心零硬编码命令名 / 步骤名 / 选项名**：SlashCommand / WizardStep / SelectItemSource trait + 注册表机制

**Internal**:

- ADR-0014 核心零硬编码斜杠命令
- ADR-0015 核心零硬编码向导步骤
- **已知偏差**（spec §12）：WizardStep::render() / SelectList::render() / TuiBackend 渲染部分直接调 ratatui API 违反"零硬编码"原则，留待 v1.4 spec D 重构

**Upgrade Notes**:

- v1.3.0 → v1.3.1 **不**破坏 eflow.yaml schema
- 6 个新斜杠命令**不**影响现有 TUI 行为

---

## [1.3.2] (TBD)

**Features**:

- **2 个 headless subcommand**：
  - `eflow session start [--config PATH] [--lang LANG]` —— 持续运行 + stdin 协议（GUI 套壳接口）
  - `eflow init` —— 委托 Wizard，0/1/2 退出码
- **7 个事件 schema 冻结**（6 原有 + `SystemReady`）—— 契约冻结 v1.3.0 起（spec B2 ADR-0017）
- **5 个 stdin action**：`send` / `end` / `level` / `lang` / `help`（JSON 一行一指令，解析失败不退出）
- **4 档 exit code**：`0`（ok）/ `1`（用户错误）/ `2`（系统错误）/ `130`（Ctrl+C）
- **stdout 永远 JSON 契约**；**stderr 永远人类可读**（tracing 走 stderr 不污染 stdout）
- **TUI 零改造**（spec B2 ADR-0016）—— TUI 仍走 spec B1 同进程 trait dispatch
- **GUI 套壳契约文档**：`docs/cli-contract.md`（7 事件 / 5 stdin / 4 exit / Python 套壳示例）
- **Python 集成测试**：`tests/gui_smoke_test.py` —— 8 步流程验证契约稳定（mock provider，不调真 LLM）

**Internal**:

- ADR-0016 TUI 零改造 + subcommand 是 headless 包装
- ADR-0017 CLI 契约冻结 v1.3.0 起
- ADR-0018 单 subcommand 模式（推翻早期"6 个独立 subcommand" 假设）
- 5 个 plan deviations：#12a-v（22 个累计）—— 详 commit message

**Upgrade Notes**:

- v1.3.1 → v1.3.2 **不**破坏 eflow.yaml schema
- 新增 `eflow session start` subcommand，**不**影响现有 TUI 行为
- GUI 团队可基于契约文档（`docs/cli-contract.md`）实现任意技术栈客户端（Python / Electron / Tauri / Web）

---

## [1.3.3] (TBD)

**Features**:

- **3 档工作流**：`SimpleWorkflow`（1 次 LLM 直接答）/ `StandardWorkflow`（3 角色 + 1 次反馈，v1.0-v1.2 既有行为）/ `AdvancedWorkflow`（3 角色 + 记忆检索）
- **5 条规则自动判定**（`Concierge::determine_workflow_level`）：多文件（≥ 3 扩展名）/ 关键词（中英 case-insensitive）/ 长度（短 < 30 / 中 30-100 / 长 ≥ 100）—— **零 LLM 成本**
- **会话级 override**：`/level simple|standard|advanced|auto`（v1.3.1 空壳实装）—— `/level auto` 清除 override，回自动判定
- **核心零硬编码档位行为**（spec C ADR-0019）：`WorkflowExecutor` trait + `WorkflowRegistry` 注册表
- **加新档位零改 core**（v1.4+ "Turbo" / "Debug" 档）—— 写 1 个 `impl` + 1 行 `register()`

**Internal**:

- ADR-0019 核心零硬编码工作流档位
- 11 deviations（#13a-k）记录在 commit messages
- `WorkflowLevel` 是 `#[non_exhaustive]` —— 外部代码加 match 必须有 `_` 分支
- `AggregatedResult` 新建在 `src/workflow/mod.rs`（v1.2 Orchestrator.execute 返 String 不变）
- `CompositeMemory` 不用 `MemoryManager` trait object —— WorkflowContext 持具体类型

**Upgrade Notes**:

- v1.3.2 → v1.3.3 **不**破坏 eflow.yaml schema
- `/level` override 会话级（重启清空，不持久化）
- `WorkflowLevel` 用 `#[non_exhaustive]` —— 外部 match 必有 `_`

---

[Unreleased]: https://github.com/sansan1983/eflow/compare/v1.0.0...HEAD
[1.3.0]: https://github.com/sansan1983/eflow/compare/v1.2.0...v1.3.0
[1.0.0]: https://github.com/sansan1983/eflow/releases/tag/v1.0.0
