# Task 4 Report: Add web_fetch tool

## What I implemented

Added the `WebFetchTool` — a URL content fetching tool supporting three output formats:

1. **`crates/qbird-code-tools/src/web_fetch.rs`** — New file implementing `WebFetchTool` with `Tool` trait, using `reqwest` for HTTP requests. Supports `text`, `markdown`, and `html` output formats. Includes `strip_html` and `html_to_markdown` helpers for content processing. 2 MB response size limit with 30s timeout.
2. **`crates/qbird-code-tools/Cargo.toml`** — Added `reqwest` dependency (workspace).
3. **`crates/qbird-code-tools/src/lib.rs`** — Added `pub mod web_fetch; pub use web_fetch::WebFetchTool;`
4. **`crates/qbird-code/src/main.rs`** — Registered `WebFetchTool` in the tool registry.
5. **`locales/zh-CN.yml`** — Added 7 i18n keys (description, result format, 5 error messages).
6. **`locales/en-US.yml`** — Added 7 i18n keys (English equivalents).

## Test results

- **cargo build:** ✓
- **cargo clippy --all-targets -- -D warnings:** ✓ (0 warnings)
- **cargo fmt --check:** ✓
- **cargo test:** ✓ (124 passed, 1 ignored — Ollama smoke requires local)

## Files changed

- `crates/qbird-code-tools/src/web_fetch.rs` (new, 308 lines)
- `crates/qbird-code-tools/Cargo.toml` (modified, +1 line)
- `crates/qbird-code-tools/src/lib.rs` (modified, +2 lines)
- `crates/qbird-code/src/main.rs` (modified, +2 lines)
- `locales/zh-CN.yml` (modified, +7 lines)
- `locales/en-US.yml` (modified, +7 lines)

## Commit

```
88e9e11 feat(tools): add web_fetch tool for fetching URL content
```

## Self-review findings

- Implementation follows the same pattern as `GlobTool` / `ListDirTool`
- `risk_level: L0` (read-only, same as `glob`, `search_code`, `list_dir`)
- Uses existing `reqwest` workspace dependency (no new external dependencies)
- i18n for all user-facing strings, English-only for internal/tracing
- HTML → markdown conversion handles `<style>`/`<script>` stripping, basic heading/paragraph/table formatting
- HTML entity decoding covers basic entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`, `&apos;`)
- No tests added per the established pattern (integration-tested through the tool registry)

## Issues or concerns

None.
