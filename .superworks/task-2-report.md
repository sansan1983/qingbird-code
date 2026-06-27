# Task 2 Report: Add glob tool

## What I implemented

Added the `GlobTool` — a file path matching tool that supports `*`, `**`, and `?` glob patterns:

1. **`crates/qbird-code-tools/src/glob.rs`** — New file implementing `GlobTool` with `Tool` trait, using `walkdir` for directory traversal and `regex_lite` for glob→regex translation
2. **`crates/qbird-code-tools/src/lib.rs`** — Added `pub mod glob; pub use glob::GlobTool;`
3. **`crates/qbird-code/src/main.rs`** — Registered `GlobTool` in the tool registry
4. **`locales/zh-CN.yml`** — Added 3 i18n keys (`tool_glob_description`, `tool_glob_no_match`, `tool_glob_count`)
5. **`locales/en-US.yml`** — Added 3 i18n keys (English equivalents)

## Test results

- **cargo build:** ✓
- **cargo clippy --all-targets -- -D warnings:** ✓ (0 warnings)
- **cargo fmt --check:** ✓
- **cargo test:** ✓ (112 passed, 1 ignored — Ollama smoke requires local)
- **Flaky test (react_loop_with_mock):** 1 passed ✓

## Files changed

- `crates/qbird-code-tools/src/glob.rs` (new)
- `crates/qbird-code-tools/src/lib.rs` (modified)
- `crates/qbird-code/src/main.rs` (modified)
- `locales/zh-CN.yml` (modified)
- `locales/en-US.yml` (modified)

## Self-review findings

- Implementation matches the brief exactly
- Used existing dependencies (`walkdir`, `regex_lite`), no new dependencies added
- Replaced `☠` placeholder from the brief with `\0` (null char) for the `**`→`.*` two-step substitution — functionally identical, avoids unicode weirdness
- `risk_level: L0` is correct (read-only tool, same as `search_code`)
- No tests added per brief (YAGNI — the glob_match helper is internal; integration tested via the tool registry pattern)
- No concerns
