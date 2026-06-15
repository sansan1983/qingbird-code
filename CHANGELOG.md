# Changelog

All notable changes to eflow will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned

- **M4.5** Config-driven LLM hardening — `Client::builder().timeout()` reading `cfg.llm.timeout_secs`; wire `retry_policy` into providers
- Multi-Subagent support
- Additional LLM providers (per design doc v4.0 §10)

---

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
