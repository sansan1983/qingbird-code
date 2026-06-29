# Changelog

All notable changes to qingbird will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.17] - 2026-06-27

### Added

- **Token 用量追踪和展示**（`/usage` 命令）
- **对话历史持久化**（SQLite 存储，`/sessions` / `/session load` 命令）
- **工具输出大小限制**（防止撑爆上下文，最大 ~4000 tokens）
- **Subagent 基础并发池**（`SubagentPool` + `execute_parallel`）

### Fixed

- **全量 i18n 审计**：确保所有用户面向字符串走 `t!()`

---

## [0.2.16] - 2026-06-27

### Added

- **Skill 插件体系**: SkillRegistry + Skill trait 注册表
- **SDD 四阶段工作流**: Proposal（含 HARD-GATE）/ Spec Review / Quality Review / Archive
- **CLI 集成**: `/sdd` 斜杠命令组

---

## [0.2.15] - 2026-06-27

### Added

- **记忆系统**: SQLite + FTS5 记忆管理器（增量同步、全文搜索、预算化读取）
- **上下文管理**: Token 预算化窗口、溢出检测（4 级压力）、自动检查点
- **ReactLoop 集成**: ContextManager 可选接入，替代粗暴 50 条截断

---

## [0.2.14] - 2026-06-27

### Added

- **3 个新工具**: glob（文件搜索）、list_dir（目录列表）、web_fetch（URL 内容抓取）
- **流式接口准备**: Provider trait 新增 `stream()` 方法 + SSE 解析器 stub
- **安全模块接线**: SecurityConfig.allowed_paths 现在实际生效，阻止写入未许可路径

### Fixed

- **Mock 测试 flaky**: 修复 TCP RST 问题 + 增加就绪信号机制，测试稳定通过

---

## [0.2.13] - 2026-06-27

### Added

- **API Key 启动检查**: 启动时检查当前 Provider 的 API Key 是否已配置，未配置时给出明确的环境变量名称提示

---

## [0.2.12] - 2026-06-27

### Changed

- **i18n 补齐**: 所有交互模式用户字符串走 `t!()`，支持中英双语
  - 系统提示词、斜杠命令说明、模型/温度切换提示、上下文截断提示等
  - 新增 16 个 i18n key（zh-CN + en-US）
- **二进制 crate 接入 i18n**: 添加 `rust-i18n` 依赖，`main.rs` 调用 `rust_i18n::i18n!()`

---

## [0.2.11] - 2026-06-27

### Added

- **交互模式 `/temperature`**: 可在对话中动态设置温度参数（0.0 ~ 2.0）

---

## [0.2.10] - 2026-06-27

### Added

- **`--temperature` CLI 参数**: 覆盖 temperature 配置（0.0 ~ 2.0）
- **启动日志**: 启动时打印 provider + model 信息

---

## [0.2.9] - 2026-06-27

### Added

- **交互模式斜杠命令**: 新增 `/help`、`/model <名称>` 命令，可在对话中动态切换模型
- **上下文窗口管理**: 消息历史超出 50 条时自动截断（保留 system 消息 + 最近一半）

---

## [0.2.8] - 2026-06-27

### Added

- **`--model` CLI 参数**: 覆盖当前 provider 的 `default_model`，支持 `qingbird --model gpt-4o --execute "..."` 临时切换模型
- **自动模型解析**: ReactLoopConfig 现在从活跃 provider 的配置中读取 `default_model`，不再固定 `deepseek-v4-pro`

---

## [0.2.7] - 2026-06-27

### Added

- **`--help` 增强**: `--provider` 参数现在显示可选值列表（deepseek/deepseek-anthropic/ollama/openai/anthropic）
- **Ollama/OpenAI/Anthropic 配置统一**: 新增 `max_retries`/`retry_backoff_ms` 字段，不再硬编码 3/1000
- **系统提示词定制**: `build_system_message` 现在包含当前 Provider 名称

---

## [0.2.6] - 2026-06-27

### Added

- **`--provider` CLI 参数**: 支持 `qingbird --provider ollama --execute "..."` 临时切换 Provider，不修改配置文件

---

## [0.2.5] - 2026-06-27

### Added

- **DeepSeek Anthropic 协议路由**: `llm.active: deepseek-anthropic` 现在使用 `DeepseekAnthropicProvider`（通过 Anthropic 兼容协议调用 DeepSeek）

---

## [0.2.4] - 2026-06-27

### Changed

- **Provider 路由**: 不再硬编码 DeepseekProvider，根据 `cfg.llm.active` 选择 Provider
  - 支持 `deepseek` / `ollama` / `openai` / `anthropic` 四种
  - `HttpLlmClient` 参数从对应 provider 配置读取
  - `qingbird.yaml` 中 `llm.active` 字段现在实际生效

---

## [0.2.3] - 2026-06-27

### Changed

- **thinking 配置从 config 读取**: `ReactLoopConfig` 新增 `thinking_enabled`/`thinking_effort` 字段
  - `ReactLoop::run()` 不再硬编码 `thinking_enabled: true`，改为使用配置值
  - binary main.rs 从 `cfg.llm.deepseek` 传入 thinking 配置
  - `qingbird.yaml` 中 `llm.deepseek.thinking_enabled` / `thinking_effort` 现在实际生效

---

## [0.2.2] - 2026-06-27

### Changed

- **交互模式改为多轮对话**: `--interactive` 不再每轮重建 messages，改为追加用户消息到已有对话历史
  - messages 在循环外初始化一次（system prompt），每轮 push user message
  - ReactLoop 返回后 messages 保留全量对话，下一轮直接复用

---

## [0.2.1] - 2026-06-27

### Changed

- **核心循环重构**: ReactLoop 从 120 行内联循环重构为状态机架构
  - 新增 `Step` 枚举 (`CallLlm` / `CallTools` / `Done`)
  - 新增 `AgentHook` trait，死循环检测和 Nudge 通过 hook 注入
  - 新增 `loop.rs` (纯决策函数) + `hooks.rs` (安全机制包装)
  - 剔除死代码: `TurnResult`, `ExecutionStrategy` 枚举
  - 外部接口 (`ReactLoop::run()`) 签名不变，所有测试照常通过

---

## [Unreleased]

### Added

- **Edit tool** (`edit` — 8th built-in tool): precise substring replacement in files with single-match enforcement, line-level diff summary via `similar` crate, `allowed_paths` and risk-level checks. `EditTool` struct with `Tool` trait implementation
- **UndoStack** (20-deep LIFO ring buffer): `UndoStack` + `UndoEntry` in `qbird-code-tools`, `Arc<Mutex<UndoStack>>` wired into `EditTool` so every successful edit auto-pushes the pre-edit file snapshot
- **`/undo` slash command** in interactive mode: pops the undo stack and writes the file back to its previous content; rejected in `--execute` mode
- **i18n keys** for edit/undo: `interactive_edit_diff_summary`, `interactive_edit_success`, `interactive_undo_success`, `interactive_help_undo`, `err_tool_edit_not_found`, `err_tool_edit_ambiguous`, `err_undo_stack_empty`, `err_undo_unavailable_in_execute`, `err_undo_lock_failed` (zh-CN + en-US)
- **12 unit tests** for EditTool (match/no-match/ambiguous/diff/allowed_paths/risk/no-file) and UndoStack (push/pop/limit/profile-preserved/execute-blocked)
- **1 integration test** for edit → undo round trip

### Changed

- **Profile system** (`--profile <name>` flag + `/profile` slash command): user profile files at `<data_dir>/qingbird/profiles/<name>.yaml` override parts of `qingbird.yaml` — `system_prompt` (replace, not append), `tools_allow` (whitelist enforced in `ToolRegistry.execute`), `risk_threshold`, `provider`, `model`. Resolution order: `--profile` CLI flag > `qingbird.yaml` `profiles.default` > no profile. Mid-session `/profile <name>`, `/profile list`, `/profile` (current). `Profile::load`, `Profile::list`, `Profile::merge_into`, `Profile::default_dir`. `ToolRegistry.set_allowed_tools` + whitelist check in `execute`. New `EflowError` variants: `ProfileNotFound { name }`, `ProfileMalformed { name, reason }`, `ToolNotAllowed { tool, allowed }` — all with `user_message()` i18n keys
- **10 unit tests** for `Profile` (load/list/merge/replace semantics/default dir), **3 tests** for `ToolRegistry` `allowed_tools` whitelist (block/admit/none-allows-all), **3 tests** for the CLI `--profile` flag path (load → merge → user_message on missing → list)

### Known limitations

- Profile `provider` and `model` fields require restart to take effect; mid-session `/profile <name>` applies other fields immediately but logs a warning (and prints to stderr) when these two fields would have changed something. The `HttpLlmClient` + `Box<dyn Provider>` are constructed at startup before the profile is merged, so a profile with `provider: ollama` would silently keep `deepseek` until a follow-up task reorders / re-inits the LLM at the profile-application point. Same caveat applies to `--profile` at startup. Tracked as a known limitation of v0.3.0; new i18n keys `interactive_profile_warn_provider` / `interactive_profile_warn_model` (both locales) provide the user-facing message.
- **Session lifecycle** (`/session delete <id>` / `/session rename <id> <name>`): delete with auto-archive to `<data_dir>/qingbird/sessions.archive/<id>.jsonl` (one line per message, JSON `role/content/timestamp`); rename persists across reopens; prefix-match deletion (exact wins, then `LIKE prefix%`, ambiguous → `SessionAmbiguous` error)
- **SessionStore new API**: `delete(id_or_prefix, archive_dir)`, `rename(id, new_name)`, `list_with_meta() -> Vec<SessionMeta>` (no LIMIT 20), `cleanup_old_sessions(keep) -> Vec<deleted_ids>`
- **SessionMeta struct**: `id / name / created_at / updated_at / message_count`
- **EflowError variants**: `SessionNotFound { id }`, `SessionAmbiguous { prefix, count }` (both with `user_message()` i18n keys)
- **Startup LRU**: on every interactive-mode launch, `cleanup_old_sessions(50)` prunes oldest-by-updated_at; silent to user, logged at `warn` on failure
- **`tempfile` dev-dep** in `qbird-code-infra` (8 new SessionStore unit tests)
- **8 unit tests + 1 integration test** for the session lifecycle (total 9 new tests)

---

## [0.2.19] - 2026-06-28

### Added

- **6 plan-shape model types**: `PermissionSet` (with `allows_tool/allows_path/allows_risk`), `Role`, `Capability`, `MemoryCategory`, `Importance` (with `Ord`), `RetryPolicy` (with `backoff_for_attempt`) — replaces 18 write-only stubs from Phase 1
- **RuntimeOverrides**: `--provider/--model/--temperature` CLI overrides immutable config; `fast` alias resolves to `cfg.llm.deepseek.fast_model`; `/provider <name>` slash command
- **Config validation** (`config_validate::validate_config`): 4 rules — `llm.active` valid, active provider `api_key` non-empty, `profiles.default` exists, `memory.working_memory_limit > 0`
- **Risk threshold**: `ToolRegistry.set_risk_threshold(RiskLevel)` replaces hardcoded L3; `/usage` shows L1 cache hit tokens when `cache.l1_enabled`
- **MemoryManager XDG default path**: `$XDG_DATA_HOME/qingbird/memory.db`, auto-create parent; removed unused `project_db_path`/`user_db_path` fields
- **RetryPolicy in HttpLlmClient**: `max_backoff_ms` cap, provider legacy `(max_retries, retry_backoff_ms)` mapped via `legacy_retry_policy()` helper
- **ContextManager wired into ReactLoop**: per-iteration `add_chat_message` + `checkpoint_if_needed`; budget-aware truncation in interactive mode (replaces 50-message hard truncate)
- **MemoryManager production integration**: per-turn `recall(query, 500)` injects `[memory]` prefix; async `save_with_summarization` (200-char clamp) on each iteration; `evict_by_importance(keep)` available
- **13 new config validation tests**, **11 RuntimeOverrides tests**, **22 models tests**, **5 risk threshold tests**, **7 memory path tests**, **6 HTTP retry tests**, **10 context manager integration tests**, **8 memory recall tests**

### Changed

- **`RequestConfig` now carries `model` field**: all 5 providers check `config.model` before `default_model` — fixes hidden bug where `--model` only updated UI display
- **`EflowConfig` explicit `Default` impls**: `LlmConfig` (active="deepseek") and `MemoryConfig` (working_memory_limit=1000) — `#[derive(Default)]` was ignoring serde defaults

---

## [0.2.18] - 2026-06-27

### Added

- **`/help` 输出刷新**: 渲染全部 7 个交互模式斜杠命令（中英双语）
- **`/sdd confirm` 接通 `hard_gate_blocked`**: SDD proposal 状态机真实生效
- **`--lang` CLI flag**: 启动时显式指定 locale（`zh-CN` / `en-US`），优先级高于 yaml `core.language`
- **`EflowError::user_message()`**: 所有错误变体走 `t!()` i18n 键；全量 i18n 审计
- **CHANGELOG 清理**: 移除历史遗留段（1.0.0 / 1.1.0 / 1.3.0-1.3.3）
- **删除 9 个 write-only 类型**: `Intent` / `IntentType` / `TaskSpec` / `TaskStep` / `TaskPlan` / `PlannedStep` / `ActionResult` / `FeedbackRecord` / `QualityVerdict`
- **`EventChannel` 占位清理**: 文件顶部标注 v0.4+ 接入方向，删除空 test mod
- **文档三件套刷新**: `CLAUDE.md` / `README.md` / `docs/architecture.md` 同步到 v0.3.0 目标
- **占位斜杠命令**: `/undo` / `/profile` / `/provider` / `/session delete` 在 `/help` 标记 `[planned]`

### Fixed

- **CLI 启动序列**: locale 初始化顺序明确，无 race condition
- **i18n 漏网**: `main.rs` 散落 `eprintln!` 全部走 `t!()`

---

[Unreleased]: https://github.com/sansan1983/qingbird/compare/v0.2.19...HEAD
[0.2.19]: https://github.com/sansan1983/qingbird/compare/v0.2.18...v0.2.19
[0.2.18]: https://github.com/sansan1983/qingbird/compare/v0.2.17...v0.2.18
[0.2.17]: https://github.com/sansan1983/qingbird/releases/tag/v0.2.17
