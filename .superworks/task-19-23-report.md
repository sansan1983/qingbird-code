# Tasks 19-23 Report (v0.2.17)

**Branch:** feature/v0.2.17-daily-usability  
**Date:** 2026-06-27  

---

## Status

| Task | Description | Status |
|------|-------------|--------|
| 19   | Token usage display (`/usage`)，LoopState 累加 + tracing logging | ✅ |
| 20   | Session persistence (SQLite `SessionStore`，`/sessions`/`/session load`) | ✅ |
| 21   | Tool output truncation (MAX_OUTPUT_TOKENS = 4000) | ✅ |
| 22   | Subagent pool (`SubagentPool` + `execute_parallel`) | ✅ |
| 23   | i18n audit + version bump 0.2.16 → 0.2.17 | ✅ |

## Commits (squashed)

All changes in a single logical commit on `feature/v0.2.17-daily-usability`.

### Files changed

| File | Action | Task |
|------|--------|------|
| `Cargo.toml` | edit: version 0.2.17 | 23 |
| `CHANGELOG.md` | edit: v0.2.17 entry | 23 |
| `locales/zh-CN.yml` | edit: +18 i18n keys + nudge_prefix | 23 |
| `locales/en-US.yml` | edit: +18 i18n keys + nudge_prefix | 23 |
| `crates/qbird-code/Cargo.toml` | edit: +uuid, chrono, dirs deps | 20 |
| `crates/qbird-code/src/main.rs` | edit: /usage, /sessions, /session load, session persistence, i18n wraps | 19, 20, 23 |
| `crates/qbird-code/tests/cli_test.rs` | edit: version check 0.2.17 | 23 |
| `crates/qbird-code-agents/src/lib.rs` | edit: +subagent_pool export | 22 |
| `crates/qbird-code-agents/src/subagent_pool.rs` | **new**: SubagentPool + execute_parallel | 22 |
| `crates/qbird-code-agents/src/react_loop/types.rs` | edit: +total_prompt/completion_tokens | 19 |
| `crates/qbird-code-agents/src/react_loop/mod.rs` | edit: token accumulation after LLM response | 19 |
| `crates/qbird-code-infra/src/memory/mod.rs` | edit: +session_store | 20 |
| `crates/qbird-code-infra/src/memory/session_store.rs` | **new**: SessionStore (SQLite) | 20 |
| `crates/qbird-code-tools/src/registry.rs` | edit: +MAX_OUTPUT_TOKENS truncation | 21 |

## Test results

```
cargo build       → OK
cargo clippy      → OK (0 warnings, -D warnings)
cargo fmt --check → OK
cargo test        → OK (139 passed, 0 failed)
```

## Concerns

1. **nudge_prefix** — this key was referenced in `nudge.rs` but missing from both locale files. Added in this PR.
2. **SessionStore path** — uses `dirs::data_dir()/qingbird/sessions.db`. Falls back to `.qingbird/` if data_dir unavailable.
3. **`/usage` accuracy** — cumulative across turns in interactive mode. Per-run stats logged via `tracing::info!`.
4. **No tests for SessionStore or SubagentPool** — existing test infrastructure doesn't cover these; manual validation required.
5. **tracing::info!(t!(...))** at main.rs:280 is an existing i18n-in-tracing inconsistency (unrelated to this PR).
