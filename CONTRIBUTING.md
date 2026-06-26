# Contributing to qingbird-code

See [CLAUDE.md](CLAUDE.md) for project status, architecture, and gates.

## Quick Reference

- **Branch rule**: Direct pushes to `main` are forbidden. All changes must go through PRs.
- **4 gates**: `cargo build` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` / `cargo test --workspace`
- **i18n**: All user-facing strings use `t!()` — add keys to both `locales/zh-CN.yml` and `locales/en-US.yml`
- **Commits**: Conventional Commits style (`feat:` / `fix:` / `chore:` / `docs:` / `refactor:` / `test:`)
