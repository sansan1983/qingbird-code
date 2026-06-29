# AGENTS.md — qingbird

> 给 AI 编程 agent 的快速参考。贡献者规则见 [`CONTRIBUTING.md`](CONTRIBUTING.md)；
> 会话交接见 [`CLAUDE.md`](CLAUDE.md)（会话开始读顶部"当前状态"表，结束更新它）。

## 这是什么

Rust 2024 5-crate workspace 二进制 Agent 框架（`qingbird` v0.2.19）。Crate 严格依赖：

```
qbird-code (bin) → qbird-code-agents → qbird-code-tools
                 → qbird-code-infra → qbird-code-models
```

下层**禁止** import 上层。要破例就在 PR 描述里说（PR 模板会强制）。

## 四个门禁（commit / PR 前必跑）

每个改动本地全过这 4 个。CI（`.github/workflows/ci.yaml`）在 push/PR 到 `main` 时也跑，顺序一致：

```bash
cargo fmt --check                                 # 第一个跑，最快反馈
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace
cargo build                                       # CI 加 --release --workspace
```

顺序按 CI：`fmt --check` → `clippy` → `test` → `build`。`clippy` 和 `test` 加 `--workspace` 避免漏 crate。

## 分支与 PR 流程（严格）

- **`main` 保护**。v1.1 起禁直推，GitHub 仓库设置里的 branch protection 强制。
- 功能分支基于 milestone：`git checkout milestone/v1.4 && git checkout -b feature/<kebab>`。
- 分支命名：小写 kebab-case，≤50 字符，动词或名词短语（不是数字）。
  前缀：`milestone/v<X>.<Y>`、`feature/*`、`fix/*`、`hotfix/*`。
- Squash-merge 到 milestone 分支。milestone → main 由 maintainer 单 PR 收尾。
- PR 模板 `.github/PULL_REQUEST_TEMPLATE.md` 是契约，checkbox 没勾齐不要 ship。尤其注意 CHANGELOG.md `[Unreleased]` 必须更新。
- 所有 crate 共享 workspace 版本号 `0.2.19`，无特殊情况不要单独改。

## 精准改动（Surgical Changes）

PR 不带无关重构。不重排/重命名/"改进"无关文件。看到 dead code 提出来——别删。匹配现有风格；项目不自定义 rustfmt。

## i18n（严格）

- 所有面向用户的字符串走 `rust_i18n::t!()`。`tracing` 日志（开发者向）保持英文，用 `tracing::info!()` 等。
- 代码注释保持英文。
- 加新 key 同时加到 **`locales/zh-CN.yml`**（默认）和 **`locales/en-US.yml`**。
- `rust_i18n::i18n!(...)` 在各 crate 的 `lib.rs` 中调用，路径指向 workspace 根 `locales/`。
- 默认 locale `zh-CN`，回退 `en-US`。

## 约定

- **命名**：`snake_case` 函数/变量，`PascalCase` 类型，`SCREAMING_SNAKE_CASE` 常量。
- **错误**：`thiserror` 枚举，集中在 `crates/qbird-code-models/src/error.rs`。库代码禁用 `anyhow`。
- **提交**：Conventional Commits，scope = 模块名（`llm` / `memory` / `react-loop` / `tools` 等）。Subject ≤72 字符，祈使句，末尾无句号。
- **涉及 LLM 的测试**必须 dummy key + 5s timeout。

- **语言**：所有推理、思考、回复必须使用简体中文。日志（tracing）保持英文。

## 文件地图（先看这些）

| 关注点 | 位置 |
|---|---|
| 二进制入口 | `crates/qbird-code/src/main.rs`（`--execute` / `--interactive`） |
| ReAct 循环 | `crates/qbird-code-agents/src/react_loop/` |
| 死循环检测 + Nudge | `crates/qbird-code-agents/src/{doom_loop,nudge}.rs` |
| Subagent | `crates/qbird-code-agents/src/subagent.rs` |
| Subagent 并发池 | `crates/qbird-code-agents/src/subagent_pool.rs` |
| Skill 插件体系 | `crates/qbird-code-agents/src/skill/` |
| LLM Provider（5 路由） | `crates/qbird-code-infra/src/providers/{deepseek,ollama,openai,anthropic}.rs` + deepseek-anthropic |
| HTTP 客户端（重试+退避） | `crates/qbird-code-infra/src/http_client.rs` |
| 配置系统 | `crates/qbird-code-infra/src/config.rs` |
| 记忆系统（SQLite+FTS5） | `crates/qbird-code-infra/src/memory/` |
| 事件/环境模块 | `crates/qbird-code-infra/src/{event,env}.rs` |
| 核心类型 | `crates/qbird-code-models/src/{types,message,error}.rs` |
| 工具系统（7 内置） | `crates/qbird-code-tools/src/`（read/write/search/command/glob/list_dir/web_fetch） |
| 工具注册表 | `crates/qbird-code-tools/src/registry.rs` |
| i18n key | `locales/zh-CN.yml`、`locales/en-US.yml` |
| 配置样例 | `qingbird.yaml`（项目根） |
