# Changelog

All notable changes to eflow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

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

[Unreleased]: https://github.com/sansan1983/eflow/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/sansan1983/eflow/releases/tag/v1.0.0
