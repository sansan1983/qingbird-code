# Tasks 15-18 Report (v0.2.16 — SDD Workflow)

## Summary

All 4 tasks implemented, verified, and passing all gates.

---

## Task 15: Skill plugin system

Created 3 files + updated lib.rs:

| File | Purpose |
|------|---------|
| `crates/qbird-code-agents/src/skill/types.rs` | `Skill` trait, `SkillDescriptor`, `SkillContext`, `SkillResult`, `SkillMetrics`, `SkillError`, `SddProposal`, `AutoTrigger` enum |
| `crates/qbird-code-agents/src/skill/registry.rs` | `SkillRegistry` — HashMap-backed registry with `register`/`get`/`list`/`match_auto`/`execute` |
| `crates/qbird-code-agents/src/skill/mod.rs` | Module root, re-exports |
| `crates/qbird-code-agents/src/lib.rs` | Added `pub mod skill;` |

**Clippy fix**: Added `impl Default for SkillRegistry` (clippy requires `new_without_default`).

---

## Task 16: SDD 4-stage workflow skills

Created 5 files under `crates/qbird-code-agents/src/skill/sdd/`:

| File | Skill ID | Blocking | AutoTrigger |
|------|----------|----------|-------------|
| `proposal.rs` | `sdd-proposal` | `true` | `Conditional` |
| `review_spec.rs` | `sdd-review-spec` | `false` | `Conditional` |
| `review_quality.rs` | `sdd-review-quality` | `false` | `Conditional` |
| `archive.rs` | `sdd-archive` | `false` | `Manual` |
| `mod.rs` | Module root + `register_all()` | | |

All 4 implement the `Skill` trait with proper descriptors and execute logic per brief spec.

---

## Task 17: CLI integration

Modified `crates/qbird-code/src/main.rs`:

1. Added `SkillRegistry` initialization after `ToolRegistry` (with SDD registration)
2. Added `/sdd` slash command handler with 3 subcommands:
   - `/sdd run [input]` — calls `sdd-proposal` skill
   - `/sdd confirm` — placeholder
   - `/sdd status` — lists registered SDD skills

Added 4 i18n keys to both `zh-CN.yml` and `en-US.yml` for the SDD help text.

---

## Task 18: Version bump

- Workspace `Cargo.toml`: `0.2.15 → 0.2.16`
- `main.rs` clap version: updated to `0.2.16`
- `CHANGELOG.md`: Added `[0.2.16]` section
- `cli_test.rs` version assertion: updated to `0.2.16`

---

## Verification Results

| Gate | Status |
|------|--------|
| `cargo build` | ✅ Pass |
| `cargo clippy --all-targets -- -D warnings` | ✅ Zero warnings |
| `cargo fmt --check` | ✅ Clean |
| `cargo test` | ✅ 136 passed, 1 ignored (Ollama smoke = expected) |

---

## Files Changed

```
M  Cargo.toml                         (0.2.15 → 0.2.16)
M  CHANGELOG.md                       (add v0.2.16 section)
M  crates/qbird-code-agents/src/lib.rs           (add pub mod skill)
A  crates/qbird-code-agents/src/skill/mod.rs      (new)
A  crates/qbird-code-agents/src/skill/types.rs     (new)
A  crates/qbird-code-agents/src/skill/registry.rs  (new)
A  crates/qbird-code-agents/src/skill/sdd/mod.rs       (new)
A  crates/qbird-code-agents/src/skill/sdd/proposal.rs  (new)
A  crates/qbird-code-agents/src/skill/sdd/review_spec.rs (new)
A  crates/qbird-code-agents/src/skill/sdd/review_quality.rs (new)
A  crates/qbird-code-agents/src/skill/sdd/archive.rs    (new)
M  crates/qbird-code/src/main.rs             (SkillRegistry + /sdd cmd)
M  crates/qbird-code/tests/cli_test.rs       (version 0.2.0→0.2.16)
M  locales/zh-CN.yml                      (SDD help keys)
M  locales/en-US.yml                      (SDD help keys)
```
