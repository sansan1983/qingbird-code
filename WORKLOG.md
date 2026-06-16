# eflow 工作日志

> 完整历史。最近 3 条在 `CLAUDE.md` 顶部，此处为归档。

---

| 日期 | 动作 | 产出 |
|------|------|------|
| 2026-06-15 | QA P0 修复 | t!()→.to_string() 56+ 处（config/llm/loader/project/compressor/blackboard/tools/registry）；config.rs 删 unsafe 块；blackboard.rs uuid 移 mod tests |
| 2026-06-15 | M7.5 i18n 国际化 | `src/infrastructure/locale.rs` + `locales/{zh-CN,en-US}.yml` + 13 处硬编码翻译 + `tests/i18n_test.rs`（10 测试） |
| 2026-06-15 | M9 能力层核心 | `src/capability/{blackboard,decisioner,executor,feedbacker}.rs` + `tests/capability_test.rs`（16 集成测试）+ Blackboard 10 inline 测试 + 7 i18n 键 |
| 2026-06-15 | M10 Subagent | `src/capability/subagent.rs` + 扩展 `tests/capability_test.rs`（+ 4 集成测试）+ 1 i18n 键 |
| 2026-06-15 | M11 Orchestrator | `src/application/{mod,orchestrator}.rs`（替换 stub）+ `tests/orchestrator_test.rs`（8 测试）+ 1 i18n 键 |
| 2026-06-15 | M12 Concierge | `src/application/concierge.rs` + 扩展 `src/application/mod.rs` + `tests/concierge_test.rs`（10 测试）+ 6 i18n 键 |
| 2026-06-15 | M13 CLI 交互 | `src/interaction/{mod,cli}.rs` + 重写 `src/main.rs` 端到端入口 + `tests/cli_test.rs`（7 测试）+ 5 i18n 键 |
| 2026-06-15 | M14 集成与端到端测试 | `tests/integration_test.rs`（11 测试）+ i18n 全局审查通过 |
| 2026-06-15 | M7 Profile + Skill | `src/infrastructure/profile/{mod,loader,skill}.rs` + `profiles/developer.yaml` + `tests/profile_test.rs` |
| 2026-06-15 | M6 上下文管理 | `src/infrastructure/context/{mod,reference,compressor}.rs` + `tests/context_test.rs`（ContextRef 从 types.rs 挪来） |
| 2026-06-15 | M4 LLM 集成 (Provider + Router) | `src/infrastructure/llm/{mod,types,anthropic,openai,router}.rs` + futures-util |
| 2026-06-15 | M3 事件通道 | `src/infrastructure/event.rs` 实装 + `tests/event_test.rs` |
| 2026-06-15 | M2 配置系统（含 Windows 兼容） | `src/infrastructure/{mod,config}.rs` + 5 stubs + `tests/config_test.rs` + `dirs` crate |
| 2026-06-15 | CLAUDE.md 重构：建立会话交接 + 铁律 + 项目速览 | `CLAUDE.md`, `WORKLOG.md` |
| 2026-06-15 | 实现计划生成 | `docs/superpowers/plans/2026-06-15-eflow-v1.0-implementation-plan.md` |
| 2026-06-15 | 架构审查 + 重构 v4.0 | `docs/superpowers/specs/2026-06-15-eflow-design.md` |
| 2026-06-16 | v1.0.3 hotfix + 工程质检 | v1.0.3 分支 (6 commits): 修真 2 bug (UTF-8 panic + task_id 'unknown') + 修 cli dead field + 8 处代码去重 (R1-R8) + 4 处 magic→const (M1+M2+M3+M5) + clippy pedantic 166 个警告 auto-fix。154/154 tests pass, 0 clippy 警告。PR #7 已开 |
| 2026-06-16 | v1.1 重开 (基于 v1.0.3) | PR #7 合入 main (c51cb67) → 删本地 v1.0.3 + 旧 v1.1 + 远端 v1.0.3 → 从 main 重开 v1.1 并 push。新 v1.1 = c51cb67（v1.0.3 完整代码），154/154 绿 |
| 2026-06-16 | v1.1 M4.5 全过 (A1-A5) | 5 commits: f662cb3 (LlmConfig 扩字段) + 5d486c7 (reqwest timeout 注入) + ca1d8b4 (Router 指数退避) + 9623bc5 (Anthropic L1 cache_control) + ae16daf (tier 降级路径)。M4.5 = 100%。A5 修 plan 同 tier bug（chat_with_retry_named 抽离），QA 报告 B1-B5 + C1-C3 仍待后续 phase |
| 2026-06-16 | v1.1 A6 收尾 + Phase A 关闭 | 1 commit: 9b6f9c6 (A6 集成测试 + i18n 键)。M4.5 = 100%。Phase A 全关。Plan 偏差：删 PathBuf unused import + 改写 `(*key).into()` 显式 3 个 t!() 避 ambiguous From<&str> 编译错 + 删局部 ModelTier use 走顶层 glob |
| 2026-06-16 | v1.1 post-PR hotfix | 1 commit: 5b8cce7 (e2e_concierge timeout 15s→30s)。触发：完工门禁 build/clippy/fmt/test 发现 1 个 flaky test（满套件 12/13，独立 5/5 全过 8s）→ A3 退避 7s 压穿 15s 边界。修后独立 10/10 + 满套件 163/163 稳定。PR #8 body 增 hotfix 行 + Post-PR Hotfix 段落 |
| 2026-06-16 | v1.1 跨阶段 + D1-D4 收尾 | 5 commits: ac66d7c (bump v1.1.0 + CHANGELOG + README) + bc4ea90 (--execute 等 TaskCompleted/Failed 事件) + 75e3f3c (base URL env var 支持) + b3cc335 (base URL 语义修正：base + /v1/messages 拼 path，SDK 兼容) + e56a99e (L2 cache 接通 Decisioner/Executor/Feedbacker)。e2e 用 minimaxi proxy 跑通：Run 1 6.6s → Run 2 0.2s (31× 加速，cache 全命中，DB 不增长)。修复 2 真 bug：base URL 缺 /v1/messages 拼接 + L2 cache 在 production 路径上死代码（M8/M9 集成 gap）。完工门禁 186/186 稳定 + 0 clippy 告警 + 0 fmt diff + 0 leftover。Push 待做 |

---

*日志始于 2026-06-15*
