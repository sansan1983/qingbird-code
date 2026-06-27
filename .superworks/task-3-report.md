# Task 3 Report: Add list_dir tool

## What I implemented

Added the `ListDirTool` — a directory listing tool:

1. **`crates/qbird-code-tools/src/list_dir.rs`** — New file implementing `ListDirTool` with `Tool` trait, using `tokio::fs::read_dir` for async directory traversal. Lists entries with `[DIR]`, `[LINK]`, or `[FILE]` prefix. Sorted alphabetically. Caps at `MAX_ENTRIES = 1000` with truncation flag.
2. **`crates/qbird-code-tools/src/lib.rs`** — Added `pub mod list_dir; pub use list_dir::ListDirTool;`
3. **`crates/qbird-code/src/main.rs`** — Registered `ListDirTool` in the tool registry
4. **`locales/zh-CN.yml`** — Added 4 i18n keys (`tool_list_dir_description`, `tool_list_dir_result`, `err_tool_not_a_directory`, `err_tool_read_dir`)
5. **`locales/en-US.yml`** — Added 4 i18n keys (English equivalents)

## Test results

- **cargo fmt --check:** ✓
- **cargo clippy --all-targets -- -D warnings:** ✓
- **cargo build:** ✓
- **cargo test:** ✓ (all 124 passed, 1 ignored — Ollama smoke requires local)

## Files changed

- `crates/qbird-code-tools/src/list_dir.rs` (new, 97 lines)
- `crates/qbird-code-tools/src/lib.rs` (modified, +2 lines)
- `crates/qbird-code/src/main.rs` (modified, +2 lines)
- `locales/zh-CN.yml` (modified, +4 lines)
- `locales/en-US.yml` (modified, +4 lines)

## Self-review findings

- Implementation follows the exact same pattern as `GlobTool` / `ReadFileTool`
- `risk_level: L0` (read-only, same as `glob`, `search_code`, `read_file`)
- Uses existing `tokio` dependency (no new dependencies)
- i18n for all user-facing strings, English-only for internal/tracing
- No tests added per the established pattern (integration-tested through the tool registry)
- No concerns
