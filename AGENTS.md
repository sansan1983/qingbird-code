# AGENTS.md — eflow

> Quick-reference for AI coding agents. For contributor/PR rules, see
> [`CONTRIBUTING.md`](CONTRIBUTING.md). For session-to-session handoff,
> see [`CLAUDE.md`](CLAUDE.md) (read its top "当前状态" table at session
> start; update it at session end). For deep architecture, see
> [`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md).

## What this is

Rust 2024 single-binary multi-layer Agent framework (`eflow` v1.3.3).
Four layers, strictly downward dependencies:

```
interaction → application → capability → infrastructure → common
```

A lower layer **must not** import from a higher one. If you need to break
this, call it out in the PR description (PR template enforces it).

## The four gates (run before any commit / PR)

Every change must pass all four locally. CI does not run — there is no
`.github/workflows/`. Reviewers run these by hand.

```bash
cargo build                                       # ~4s release, dev faster
cargo clippy --all-targets -- -D warnings        # zero warnings required
cargo fmt --check                                 # rustfmt defaults; no custom config
cargo test                                        # 334 tests as of v1.3.3
```

Order matters for fast feedback: `fmt --check` → `clippy` → `test`. The
`scripts/verify-v1.3.1.sh` block in `docs/manual-verification-v1.3.1.md`
is a copy-pasteable version of this.

## Branch + PR workflow (strict)

- **`main` is protected.** Direct push forbidden since v1.1. GitHub
  branch protection enforces this.
- Feature work targets a milestone branch: `git checkout milestone/v1.3
  && git checkout -b feature/<kebab>`.
- Branch names: lowercase kebab-case, ≤ 50 chars, verb or noun-phrase
  (not a number). Prefixes: `milestone/v<X>.<Y>`, `feature/*`, `fix/*`,
  `hotfix/*`.
- Squash-merge to the milestone branch. Milestone → main is a single
  PR by the maintainer.
- PR template `.github/PULL_REQUEST_TEMPLATE.md` is the contract; don't
  ship a PR that fails any of its checkboxes.

## Surgical changes

No unrelated refactors in a PR. Don't reformat, rename, or "improve"
files you didn't need to touch. If you spot dead code, mention it — don't
delete it. Match existing style; the project does not customize rustfmt.

## i18n (strict)

- All user-facing strings go through `rust_i18n::t!()`. Tracing logs
  (developer-facing) stay in English and use `tracing::info!()` etc.
- Code comments stay in English.
- When adding a key, add it to **both** `locales/zh-CN.yml` (default)
  and `locales/en-US.yml`. `tests/i18n_test.rs` enforces this.
- `rust_i18n::i18n!("locales", fallback = "en-US");` must be called in
  **both** `src/lib.rs` and `src/main.rs` (so `t!()` works in both).
- Default locale is `zh-CN`; fallback is `en-US`.
- Locale tests must be `#[serial_test::serial]` — `rust-i18n` uses
  process-global state and `cargo test`'s parallel test runner will
  poison it. This is the only quirk that needs `serial_test`.

## Stdio contract (frozen v1.3.0+)

- **stdout** = NDJSON events for `eflow session start` (GUI consumer).
  See `docs/cli-contract.md`. Any change requires an ADR.
- **stderr** = human-readable logs. `tracing-subscriber` is initialized
  with `.with_writer(std::io::stderr)` in `main.rs` — keep it that way.
- Exit codes: 0 / 1 / 2 / 130 (Ctrl+C).

## Conventions

- **Naming**: `snake_case` fns/vars, `PascalCase` types,
  `SCREAMING_SNAKE_CASE` consts.
- **Errors**: `thiserror` enums in `src/common/error.rs`. No `anyhow` in
  library code.
- **Commits**: Conventional Commits, scope = `M<n>` for milestone work
  or module name (`llm`, `memory`, `tui`). Subject ≤ 72 chars,
  imperative, no trailing period.
- **Blackboard pattern** (`src/capability/blackboard.rs`): immutable
  `with_*` updates. Don't add `&mut self` to blackboard methods.
- **LLM test helpers** (`src/infrastructure/llm/router.rs`):
  `placeholder()`, `inject_test_provider()`, `inject_test_routing()` are
  `#[doc(hidden)]` and **only for tests**. Non-test code must use
  `LlmRouter::from_config`.
- **LLM-touching tests** must use a dummy key + 5s timeout. Pattern in
  `tests/integration_test.rs` — copy it for any new end-to-end test.

## Gotchas that bite agents

- **TUI requires a real TTY.** ratatui panics without one. Headless
  CI / sandboxed environments cannot exercise the TUI; for those, use
  `eflow session start` (NDJSON on stdout) or the 14-step manual
  verification in `docs/manual-verification-v1.3.1.md`.
- **No clap `SubCommand` enum yet.** `src/main.rs` routes
  `init` / `session start` via `std::env::args()` and a hand-rolled
  flag parser (`parse_session_flag`). The plan in
  `docs/superpowers/plans/2026-06-18-eflow-v1.4-rendering-pipeline-plan.md`
  is to introduce clap derive in v1.4 — don't refactor this in a
  v1.3.x patch.
- **v1.3.0 broke `eflow.yaml`** (see `docs/migration-v1.2-to-v1.3.md`).
  `llm.providers` is gone. Providers live in
  `~/.eflow/providers/<id>.yaml`; `routing.{strong,medium,light}` now
  references those provider ids, not "anthropic"/"openai". New code
  must not add the old field back.
- **`Cargo.lock` is gitignored** by project convention. Don't commit
  it; contributors regenerate on clone.
- **Default locale is `zh-CN`.** The `eflow.yaml` example in README is
  the v1.2 form — it still works for the `routing` block but the
  `llm.providers` block is silently ignored post-v1.3.0. Use the
  v1.3 form when documenting config.
- **v1.3.3 added two registries** wired up in `main.rs::register_*`:
  6 slash commands (`model` / `profile` / `lang` / `level` / `help` /
  `quit`) and 3 workflow levels (`Simple` / `Standard` / `Advanced`).
  Both use a `required_register` check — don't add a new entry to the
  registry without listing it in the required set.
- **v1.3.1 has a known deviation** (`TODO(v1.4 spec D)` markers in
  `src/interaction/wizard/mod.rs` and `src/interaction/tui.rs`):
  wizard/SelectList/TUI call ratatui directly instead of going through
  a `RenderEngine` trait. The v1.4 spec D plan fixes this. Don't try
  to fix it as a drive-by.

## File map (where to look first)

| Concern | Location |
|---|---|
| Entry point | `src/main.rs` (TUI default, `--execute`, `--show-config`, `--list-profiles`, `init`, `session start`) |
| TUI backend | `src/interaction/tui.rs` |
| Wizard steps | `src/interaction/wizard/builtin/*.rs` |
| Slash commands | `src/interaction/slash/builtin/*.rs` |
| Workflow levels | `src/workflow/builtin/{simple,standard,advanced}.rs` |
| Concierge (zero-blocking dispatch) | `src/application/concierge.rs` |
| Orchestrator (decompose + parallel by layer) | `src/application/orchestrator.rs` |
| D→E→F pipeline | `src/capability/{decisioner,executor,feedbacker}.rs` |
| Subagent pool | `src/capability/pool.rs`, `subagent.rs` |
| LLM router | `src/infrastructure/llm/router.rs` |
| Provider presets | `~/.eflow/providers/<id>.yaml` (user) / `docs/examples/providers/` (samples) |
| i18n keys | `locales/zh-CN.yml`, `locales/en-US.yml` |
| CLI / GUI stdio contract | `docs/cli-contract.md` |
| Manual TUI verification | `docs/manual-verification-v1.3.1.md` |
| Next milestone plan (v1.4) | `docs/superpowers/plans/2026-06-18-eflow-v1.4-rendering-pipeline-plan.md` |
