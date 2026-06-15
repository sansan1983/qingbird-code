# Contributing to eflow

Thank you for your interest in contributing! eflow is a Rust-native multi-layer Agent collaboration framework. This document explains how to participate.

---

## ⚠️ Strict Branch Rule (v1.1+)

> **Direct pushes to `main` are FORBIDDEN. All changes must go through a feature branch + Pull Request workflow.**

This rule is enforced by GitHub branch protection on `main`. Even the project maintainer opens PRs for their own work.

The v1.0 release is the only direct commit to `main`. From v1.1 onwards:

```
┌──────────────────────────────────────────────────────────┐
│  main ←──  protected branch (PR-only)                    │
│    ↑                                                     │
│    └── milestone/v1.x  ←── feature/<name>                │
│           ↑                    ↑                         │
│           └── fix/<name> ──────┘                         │
└──────────────────────────────────────────────────────────┘
```

### Branch Naming Convention

| Prefix | Use case | Example |
|--------|----------|---------|
| `milestone/v<X>.<Y>` | Milestone baseline branches (long-lived) | `milestone/v1.1` |
| `feature/<short-name>` | New features within a milestone | `feature/llm-timeout` |
| `fix/<short-name>` | Bug fixes (non-urgent) | `fix/profile-load-error` |
| `hotfix/<short-name>` | Urgent fixes that must skip the milestone branch | `hotfix/critical-crash` |

Branch names use **lowercase kebab-case** and should be ≤ 50 characters. The short name should be a verb or noun-phrase, not a number.

---

## Pull Request Workflow

1. **Sync with the milestone branch**
   ```bash
   git fetch origin
   git checkout milestone/v1.1
   git pull
   git checkout -b feature/your-feature
   ```

2. **Develop & commit** (see [Commit Message Format](#commit-message-format) below)

3. **Push your branch**
   ```bash
   git push -u origin feature/your-feature
   ```

4. **Open a Pull Request** on GitHub targeting `milestone/v1.1` (not `main`)

5. **Pass CI + Reviewer approval** before merge

6. **Squash-merge** to keep `milestone/v1.x` history linear

7. **Milestone → main**: When a milestone is complete, the maintainer opens a single PR from `milestone/v1.x` → `main`

### PR Requirements (enforced by the template)

- [ ] `cargo fmt` clean
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test` all passing
- [ ] New code has tests
- [ ] i18n: all new user-facing strings go through `t!()` with both `zh-CN` and `en-US` keys
- [ ] CHANGELOG.md updated (under "Unreleased")
- [ ] No unrelated refactors ("Surgical Changes")
- [ ] Linked to a GitHub issue or milestone

---

## Local Development Setup

### Prerequisites

- Rust 2024 edition (`rustup default stable`, stable ≥ 1.85)
- An LLM API key (Anthropic or OpenAI)
- `git`, `make` (optional)

### Setup

```bash
git clone https://github.com/sansan1983/eflow.git
cd eflow

# Copy the example config
cp eflow.yaml.example eflow.yaml   # or use the template from README

# Export an API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Build
cargo build

# Run tests
cargo test
```

### Useful Commands

```bash
# Format
cargo fmt

# Lint (CI runs this with -D warnings)
cargo clippy --all-targets -- -D warnings

# Test
cargo test

# Run a specific test
cargo test test_name

# Run with logging
RUST_LOG=eflow=debug cargo run
```

---

## Code Style

- **Formatting**: `cargo fmt` (rustfmt defaults; we do not customize)
- **Linting**: `cargo clippy --all-targets -- -D warnings`
- **Idioms**: Prefer iterators over indexing; use `?` for error propagation; use `thiserror` for error enums
- **Naming**: `snake_case` for functions/variables; `PascalCase` for types; `SCREAMING_SNAKE_CASE` for constants
- **Comments**: Comments in code stay in English; commit messages in English; user-facing strings via `t!()`

### Architecture Boundaries

Respect the four-layer architecture — **dependencies flow downward only**:

```
interaction/ → application/ → capability/ → infrastructure/ → common/
```

A lower layer MUST NOT import from a higher layer. If you need to break this, raise it in the PR description.

---

## Internationalization (i18n)

eflow supports zh-CN (default) and en-US. **All user-facing strings** must go through `t!()`:

```rust
// ❌ Don't hardcode
println!("Loaded profile: {}", name);

// ✅ Use t!()
println!("{}", t!("status_profile_loaded", name = name));
```

When you add a new key:

1. Add it to `locales/zh-CN.yml` (default language)
2. Add the English translation to `locales/en-US.yml`
3. Tests in `tests/i18n_test.rs` will catch missing keys

Tracing logs (developer-facing) stay in English and use `tracing::info!()` etc.

---

## Commit Message Format

We loosely follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: `feat` | `fix` | `chore` | `test` | `docs` | `refactor` | `perf`

**Scope** (optional): `M<n>` for milestone work, or module name (e.g. `llm`, `memory`)

**Examples**:

```
feat(M4): add Anthropic LLM provider
fix(QA): convert t!() Cow<str> to String at 55+ call sites
chore: update handoff log — v1.0 closed
docs: add README.md bilingual
```

Subject line ≤ 72 chars, imperative mood, no trailing period.

---

## Testing

- Unit tests live alongside the code they test (`#[cfg(test)] mod tests`)
- Integration tests live in `tests/`
- All tests must pass on Windows, Linux, and macOS
- Tests that need an LLM key use a dummy key + 5s timeout (see `tests/integration_test.rs`)

---

## Reporting Issues

Use the appropriate issue template:

- **Bug report**: `.github/ISSUE_TEMPLATE/bug_report.md`
- **Feature request**: `.github/ISSUE_TEMPLATE/feature_request.md`

---

## Code of Conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/). By participating, you agree to abide by its terms.

---

## License

By contributing, you agree that your contributions will be dual-licensed under MIT and Apache 2.0, at the project's option (same license as the project).
