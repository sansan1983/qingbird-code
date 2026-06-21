# AGENTS.md — eflow

> 给 AI 编程 agent 的快速参考。贡献者规则见 [`CONTRIBUTING.md`](CONTRIBUTING.md)；
> 会话交接见 [`CLAUDE.md`](CLAUDE.md)（会话开始读顶部"当前状态"表，结束更新它）；
> 深度架构见 [`docs/superpowers/specs/2026-06-15-eflow-design.md`](docs/superpowers/specs/2026-06-15-eflow-design.md)。

## 这是什么

Rust 2024 单二进制多层 Agent 框架（`eflow` v1.3.3）。四层严格向下依赖：

```
interaction → application → capability → infrastructure → common
```

下层**禁止** import 上层。要破例就在 PR 描述里说（PR 模板会强制）。

## 四个门禁（commit / PR 前必跑）

每个改动本地全过这 4 个。**没有 CI**（项目无 `.github/workflows/`），reviewer 手跑：

```bash
cargo build                                       # release 约 4s，dev 更快
cargo clippy --all-targets -- -D warnings        # 零警告
cargo fmt --check                                 # rustfmt 默认；无自定义配置
cargo test                                        # v1.3.3 时 335 个测试
```

顺序影响反馈速度：`fmt --check` → `clippy` → `test` → `build`。`docs/manual-verification-v1.3.1.md` 里有 `scripts/verify-v1.3.1.sh` 的复制粘贴版本。

## 分支与 PR 流程（严格）

- **`main` 保护**。v1.1 起禁直推，GitHub 仓库设置里的 branch protection 强制。
- 功能分支基于 milestone：`git checkout milestone/v1.4 && git checkout -b feature/<kebab>`。
- 分支命名：小写 kebab-case，≤50 字符，动词或名词短语（不是数字）。
  前缀：`milestone/v<X>.<Y>`、`feature/*`、`fix/*`、`hotfix/*`。
- Squash-merge 到 milestone 分支。milestone → main 由 maintainer 单 PR 收尾。
- PR 模板 `.github/PULL_REQUEST_TEMPLATE.md` 是契约，checkbox 没勾齐不要 ship。

## 精准改动（Surgical Changes）

PR 不带无关重构。不重排/重命名/"改进"无关文件。看到 dead code 提出来——别删。匹配现有风格；项目不自定义 rustfmt。

## i18n（严格）

- 所有面向用户的字符串走 `rust_i18n::t!()`。`tracing` 日志（开发者向）保持英文，用 `tracing::info!()` 等。
- 代码注释保持英文。
- 加新 key 同时加到 **`locales/zh-CN.yml`**（默认）和 **`locales/en-US.yml`**。`tests/i18n_test.rs` 强制。
- `rust_i18n::i18n!("locales", fallback = "en-US");` 必须在 **`src/lib.rs` 和 `src/main.rs` 都调用**（`main.rs` 里 `t!()` 才能用）。
- 默认 locale `zh-CN`，回退 `en-US`。
- locale 相关测试必须 `#[serial_test::serial]`——`rust-i18n` 用 process-global 状态，`cargo test` 并发跑会污染。这是项目里**唯一**需要 `serial_test` 的坑。

## stdio 契约（v1.3.0+ 冻结）

- **stdout** = NDJSON 事件，给 `eflow session start`（GUI 消费者）用。见 `docs/cli-contract.md`。任何改动要过 ADR。
- **stderr** = 人类可读日志。`tracing-subscriber` 在 `main.rs` 用 `.with_writer(std::io::stderr)`——保持。
- 退出码：`0` / `1` / `2` / `130`（Ctrl+C）。

## 约定

- **命名**：`snake_case` 函数/变量，`PascalCase` 类型，`SCREAMING_SNAKE_CASE` 常量。
- **错误**：`thiserror` 枚举，集中在 `src/common/error.rs`。库代码禁用 `anyhow`。
- **提交**：Conventional Commits，scope = `M<n>`（里程碑工作）或模块名（`llm` / `memory` / `tui`）。Subject ≤72 字符，祈使句，末尾无句号。
- **Blackboard 模式**（`src/capability/blackboard.rs`）：不可变 `with_*` 更新。Blackboard 方法**不要**加 `&mut self`。
- **LLM 测试 helper**（`src/infrastructure/llm/router.rs`）：`placeholder()` / `inject_test_provider()` / `inject_test_routing()` 是 `#[doc(hidden)]`，**只供测试**。非测试代码必须用 `LlmRouter::from_config`。
- **涉及 LLM 的测试**必须 dummy key + 5s timeout。模式在 `tests/integration_test.rs`——新加端到端测试照抄。

## 容易踩的坑

- **TUI 要真 TTY**。ratatui 没 TTY 直接 panic。无头 CI / 沙箱环境跑不了 TUI；那种情况用 `eflow session start`（NDJSON 在 stdout）或 `docs/manual-verification-v1.3.1.md` 的 14 步手工验证。
- **还没有 clap `SubCommand` enum**。`src/main.rs` 用 `std::env::args()` 路由 `init` / `session start`，外加手写 flag 解析器（`parse_session_flag`）。`docs/superpowers/plans/2026-06-18-eflow-v1.4-rendering-pipeline-plan.md` 计划 v1.4 引入 clap derive——v1.3.x patch 别重构这块。
- **v1.3.0 改了 `eflow.yaml`**（见 `docs/migration-v1.2-to-v1.3.md`）。`llm.providers` 删了。Provider 改在 `~/.eflow/providers/<id>.yaml`；`routing.{strong,medium,light}` 现在引用 provider id，不再是 `"anthropic"` / `"openai"`。新代码**不要**加回老字段。
- **`Cargo.lock` 在 `.gitignore`**。别 commit；贡献者 clone 后重新生成。
- **默认 locale 是 `zh-CN`**。README 里 `eflow.yaml` 例子还是 v1.2 形态——`routing` 块还能用，但 `llm.providers` 块 v1.3.0+ 被静默忽略。文档示例用 v1.3 形态。
- **v1.3.3 加了 slash command registry**（`main.rs::register_slash_commands` 接线）：6 个斜杠命令（`model` / `profile` / `lang` / `level` / `help` / `quit`）。用 `required_register` 校验——加新条目必须列在 required 集合里。
- **v1.3.3 spec C 实施未接通**——`/level simple` 是 no-op（`/level` 命令改占位 stub，v1.4+ 重写）。`src/workflow/` 已删（PR #21），3 档抽象整套移除，承认回退。
- **v1.3.1 有已知偏差**（`src/interaction/wizard/mod.rs` 和 `src/interaction/tui.rs` 有 `TODO(v1.4 spec D)` 标记）：wizard / SelectList / TUI 直接调 ratatui，没走 `RenderEngine` trait。v1.4 spec D 计划会修。**不要**顺手修这个。

## 文件地图（先看这些）

| 关注点 | 位置 |
|---|---|
| 入口 | `src/main.rs`（TUI 默认、`--execute`、`--show-config`、`--list-profiles`、`init`、`session start`） |
| TUI 后端 | `src/interaction/tui.rs` |
| 向导步骤 | `src/interaction/wizard/builtin/*.rs` |
| 斜杠命令 | `src/interaction/slash/builtin/*.rs` |
| ~~工作流档位~~ | ~~`src/workflow/builtin/{simple,standard,advanced}.rs`~~ （v1.3.3 spec C 实施未接通，PR #21 已删） |
| Concierge（零阻塞派发） | `src/application/concierge.rs` |
| Orchestrator（按层分解 + 并发） | `src/application/orchestrator.rs` |
| D→E→F 管线 | `src/capability/{decisioner,executor,feedbacker}.rs` |
| Subagent 池 | `src/capability/pool.rs`、`subagent.rs` |
| LLM 路由 | `src/infrastructure/llm/router.rs` |
| Provider 预设 | `~/.eflow/providers/<id>.yaml`（用户） / `docs/examples/providers/`（样例） |
| i18n key | `locales/zh-CN.yml`、`locales/en-US.yml` |
| CLI / GUI stdio 契约 | `docs/cli-contract.md` |
| 手工 TUI 验证 | `docs/manual-verification-v1.3.1.md` |
| 下一里程碑计划（v1.4） | `docs/superpowers/plans/2026-06-18-eflow-v1.4-rendering-pipeline-plan.md` |
