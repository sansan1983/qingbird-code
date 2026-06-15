## Summary

<!-- One or two sentences: what does this PR do and why? -->

## Type of Change

<!-- Check all that apply -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactor (no functional change)
- [ ] Test addition/update
- [ ] Chore (build, CI, dependencies)

## Milestone / Branch

- Target branch: <!-- e.g. `milestone/v1.1` (NEVER `main` for feature work) -->
- Milestone: <!-- e.g. `v1.1` or `v1.2` -->
- Related issue: <!-- e.g. `#42` or `N/A` -->

## Changes

<!-- Bullet list of what changed. Link to files with `path/to/file.rs:line`. -->

-

## Testing

- [ ] I added tests for the new behavior
- [ ] Existing tests still pass locally (`cargo test`)
- [ ] I tested on at least one of: Windows / Linux / macOS
- [ ] LLM-required paths use a 5s timeout guard (no production hangs)

## Checklist (CI will verify)

### Code quality

- [ ] `cargo fmt` produces no changes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] No unrelated refactors included (Surgical Changes)

### i18n

- [ ] All new user-facing strings use `t!()` macro
- [ ] Added the new key to both `locales/zh-CN.yml` and `locales/en-US.yml`
- [ ] No hardcoded Chinese or English in user-facing output

### Documentation

- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] `CLAUDE.md` handoff table updated if this closes a milestone
- [ ] New public API documented in inline doc comments (`///`)

### Architecture

- [ ] Respects 4-layer unidirectional dependencies (`interaction → application → capability → infrastructure → common`)
- [ ] No new cross-layer imports from a lower to a higher layer
- [ ] Blackboard value-type pattern preserved (immutable `with_*` updates)

## Screenshots / Output

<!-- If CLI-visible change, paste the actual output: -->

```

```

## Linked Issues

<!-- e.g. Closes #42, Fixes #15, Related to #8 -->
