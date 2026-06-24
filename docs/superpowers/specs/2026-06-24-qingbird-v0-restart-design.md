# qingbird-code V0.1.0 重启设计

> 日期：2026-06-24
> 状态：设计中，待复审
> 作者：用户 + Claude（brainstorming 协作）
> 关联：根因分析见 `docs/analysis-2026-06-24-eflow-polish.md`

---

## 一、背景

eflow 自 v1.0 至 v1.4 共 11,234 行代码 / 174 tests / 4 门禁全过 — 表面"健康"。
但用户实际体验是：**"项目越搞越乱，修了这个 bug 那个又不满意"**。

根因有三：

1. **文档凌乱碎** — 6 spec + 8 plan + 多份 ADR，文档比代码重。下次"该做什么"散在 14 份历史文档里。
2. **"为未来预留"的代码占认知负担** — `UserMemory` 别名 / `RenderEngine` trait 单 impl / `LlmRouter::placeholder()` 在生产路径上跑 / 注释里到处是"v1.5+ / v2.0"。
3. **LLM 真链路从未端到端跑通** — v1.0~v1.4 所有版本号都升了，但**没有人手动验证过 deepseek 真链路**。修一处暴露一处，说明 LLM 模块本身有结构问题。

`docs/analysis-2026-06-24-eflow-polish.md` 是这次重启的诊断依据，但**判断已被本 spec 吸收**，文件本身 V0.1.0 收尾时可删。

---

## 二、目标

**V0.1.0 收尾标准**：用户在 CLI 模式下，`qingbird --execute "..."` 走真 deepseek，decisioner/executor/feedbacker 三角色端到端协作，**手动跑 3 次都能拿到非 echo 的合理输出**。

eflow V0.1.0 **不是** v1.4 的补丁版本，而是**重新定义起跳线**：
- 之前所有版本号（v1.0~v1.4）作废，新仓库从 V0.1.0 起
- API 还会变，别锁版本
- "为未来预留"在 V0.x 阶段是反模式

---

## 三、命名与仓库

| 项 | 值 |
|---|---|
| 英文名 | `qingbird-code` |
| 中文名 | 青鸟 |
| Cargo package | `qingbird-code` |
| 二进制名 | `qingbird` |
| CLI 调用 | `qingbird --execute "..."` |
| GitHub 仓库 | https://github.com/sansan1983/qingbird-code |
| 旧 eflow 仓库 | 改名为 `qingbird-code-archive`（GitHub 自动 redirect） |
| 配置文件路径 | `~/.eflow/` **不变**（V0.1.0 范围内） |
| local 目录 | `F:\AI data\Claude Code\eflow\` **不变**（只是字符串） |

**为什么不改 `~/.eflow/`**：用户的 L2 cache SQLite 已积累数据，重跑 deepseek 填 cache 浪费 token。路径只是字符串，没人在乎。V0.x 后期需要时再做迁移。

---

## 四、范围（按版本分）

### V0.1.0 — 本 spec 覆盖

**目标**：LLM 模块瘦身 + deepseek 真链路跑通。

**改动**（实施前已扫描校正 — 实际比 spec 假设更复杂）：

1. **LLM 模块瘦身**（`src/infrastructure/llm/`）
   - 拆 `router.rs` (897 行) → `tier.rs` + `retry.rs` + `lifecycle.rs`
   - 拆 `cache.rs` (595 行) → `l1.rs` (in-memory LRU) + `l2.rs` (SQLite) + `cache_key.rs`
   - **删** `preset_loader.rs` (206 行) — 整套 yaml 扫描器废
   - **删** `registry.rs` (128 行) — multi-provider 抽象废
   - **删** `generic_anthropic.rs` (237 行) — deepseek 走 OpenAI 协议
   - **改** `generic_openai.rs` → 改名为 `deepseek.rs`，hard-code base_url = `https://api.deepseek.com`
   - 简化 `types.rs`：删 `ProviderConfig` (12 字段) / 删 `ProtocolKind` (2 变体) / 简化 `LlmProvider` trait (9 方法 → 3 方法 + `#[doc(hidden)]`)
   - 简化 `mod.rs`：删 preset_loader/registry/generic_anthropic/re-export
   - 删 `LlmRouter::placeholder()` (4 个 `inject_test_*` 配套删)
   - 删 `Concierge::placeholder()` + 8+ 测试改 mock 构造
   - 删 `Feedbacker::new_for_test()` placeholder 调用

2. **配置层简化**（`src/infrastructure/config.rs` + `src/cli/`）
   - 简化 `LlmConfig`：`providers` 子结构 → `deepseek: DeepseekConfig { api_key, base_url, default_model, timeout_secs, max_retries, retry_backoff_ms }`
   - 删 `src/cli/config.rs::write_provider()` + `check_llm_configured()` 的目录扫描
   - 删 `src/cli/start.rs:77` 的 `provider_dir` 构造 + `LlmRouter::from_config()` 签名简化（不传 provider_dir）

3. **测试改造**（不删）
   - 8+ 测试从 `placeholder()` 改为传 mock（mock 用 test-double，不引第三方 mock 框架）
   - fixture 文件 (`tests/fixtures/providers/*.yaml`) 删 — 整套 preset 概念废
   - 保留 `tests/fixtures/mock_config.yaml` 但简化 `llm.providers.*` → `llm.deepseek.*`
   - 保留 `tests/gui_smoke_test.py` 不动
   - 集成测试 (`tests/integration_test.rs`) 行为不变，只改构造方式

4. **真链路 smoke test**
   - 用户手动跑 3 次 `qingbird --execute "..."`，每次拿真 deepseek 返回

**完全不动**：
- `src/capability/{decisioner,executor,feedbacker}.rs`（3 角色代码本身）
- `src/application/orchestrator.rs`
- `src/main.rs`（除字段名变化 + `LlmRouter::from_config` 签名变化）
- `tests/` 下的测试逻辑（只改构造）

### V0.1.1 — 本 spec 不覆盖

v1.4 架构回退：删 `RenderEngine` trait + `RenderBackend` trait + `DefaultRenderEngine`，TuiBackend 直接持有 `execute_draw_commands`。保留 `ViewModel` 5 struct + `DrawCommand` enum + `state_to_vm()` + `execute_draw_commands` helper。

### V0.1.2 — 本 spec 不覆盖

文档归档：6 spec 合并为 `docs/architecture.md`，8 plan 合并为 `docs/roadmap.md`，旧的挪 `docs/archive/`。

---

## 五、验收门禁

每步实施必须：

```
cargo build ✓
cargo clippy --all-targets -- -D warnings ✓
cargo fmt --check ✓
cargo test ✓
```

外加 V0.1.0 收尾的手工 smoke test：

```bash
DEEPSEEK_API_KEY="sk-..." cargo run -- --execute "用 rust 写一个 hello world"
# 跑 3 次，每次都拿非 echo 内容
```

---

## 六、实施步骤

| 步 | 动作 | 备注 |
|---|------|------|
| 1 | 旧 eflow 仓库 commit analysis + 本 spec | 决策记录保留在 archive |
| 2 | local 复制：cp eflow 内容到 qingbird-code/ | 不含 .git |
| 3 | 批量改名：sed 替换 `eflow` → `qingbird-code`（代码/文档/config） | 93 处 `use eflow::`（已 grep 确认）+ README + Cargo.toml |
| 4 | 4 门禁全过 | 改名不应破坏编译 |
| 5 | 第一个实质 commit | "chore: eflow → qingbird-code 重命名" |
| 6 | V0.1.0 实施：拆 router/cache + 删 4 家 + 删 placeholder | 一个或几个 commit |
| 7 | 第二个 commit | "feat(V0.1.0): deepseek 真链路跑通" |
| 8 | 推 GitHub 新仓库 | `git push -u origin main` |
| 9 | 旧 eflow 仓库 archive | GitHub Settings → Archive |
| 10 | 收尾文档：CLAUDE.md (10 行) + README.md (30 行) + ADR-0018 + docs/architecture.md | 单独 commit |

---

## 七、设计原则（V0.x 阶段）

写入 ADR-0018，作为 V0.x 阶段的判断尺子：

> **V0.x 阶段判断"预留是否合理"的标准**：
> - **对的预留**：兼容旧配置（向后兼容）/ 抽象的多种实现**已经存在 2 个或更多** / 必要的类型扩展点
> - **错的预留**：只有 1 个实现还在抽象 / 注释写"v2.0 / v1.5+ / 未来" / placeholder 在生产路径上跑 / 抽象的多种实现**不存在**且**短期内不会存在**

**新模块开发规则**：写第二个实例前，**不要**抽 trait。第一次实现用具体类型 / 函数。出现第二个实例时，再抽公共接口。

---

## 八、不做（明确排除）

- ❌ 不加 L2 cache 持久化升级（向量搜索 / stats 接线）
- ❌ 不动 TUI 内部逻辑（V0.1.1 才做架构回退）
- ❌ 不动 GUI / Web / Tauri 路径
- ❌ 不写新 spec（V0.1.0 直接走 plan）
- ❌ 不为"未来"加任何 trait / 抽象 / placeholder
- ❌ 不动 `~/.eflow/` 路径
- ❌ 不删任何 tests/ 文件（V0.1.0 期间只改不删）

---

## 九、风险与已知问题

| 风险 | 应对 |
|------|------|
| 改名 93 处 import 漏改导致编译失败 | 4 门禁第一时间验证，cargo 报哪改哪 |
| 删 PresetLoader/Registry/ProviderConfig 后想加回 | git log 在 archive 仓库，spec/plan 也保留，重写不是事 |
| L2 cache 旧的 cache_key 与新代码不兼容 | V0.1.0 接受 cache 失效重填（一次性损失） |
| 用户 `~/.eflow/` 现有 provider yaml | 路径加载代码删了后 yaml 不会被读 — 这是预期行为，yaml 文件留着不影响 |
| deepseek 协议变（OpenAI 协议字段调整） | base_url + model hard-code 写死，v0.x 阶段假定协议稳定 |
| 简化 LlmProvider trait (9→3 方法) 后 capability 层某处用了别的方法 | 实施前先 grep capability 层调用，必要时 trait 方法保留 |

---

## 十、ADR-0018 草稿

写入 `docs/adr/0018-v0-restart.md`：

```markdown
# ADR-0018: eflow → qingbird-code V0.x 重启

## 当时怎么想的
v1.0 → v1.4 一路设计，4 门禁全过，174 tests pass。看起来"健康"。

## 后来发现（2026-06-24 复盘）
- 11,234 行代码，约 20% 是"为未来预留"的代码
- 6 spec + 8 plan，文档比代码重
- /level / SlashOutput 占位变体 / UserMemory 别名是摆设
- Concierge::placeholder() / LlmRouter::placeholder() 在测试里被 8+ 次用
- 注释里到处是"v1.5+ / v2.0 / 未来 / 占位"
- LLM 真链路从未端到端手动跑通验证

## 现在怎么办
- 项目改名：eflow → qingbird-code（青鸟）
- 版本重定：v1.x → V0.1.0（之前没接通真链路，不算 1.x）
- 砍掉"为未来预留"的占位（V0.1.0）
- 物理删 4 家 provider，只留 deepseek
- 删 placeholder() 抽象，让测试和生产走完全不同的代码路径
- 保留 ViewModel + DrawCommand，砍 RenderEngine/Backend trait 单实例抽象

## 教训
- "门禁全过" ≠ "系统健康"
- "tests pass" ≠ "功能完整"（测试可以测摆设）
- "为未来设计"在反复犯（spec C 撤回 / v1.4 trait 化）
- 真正健康的标志：**所有接通的代码都有人用，所有有人用的代码都接通**

## 原则
V0.x 阶段：新模块写第二个实例前，不要抽 trait。
```

---

## 十一、与 root cause analysis 的对应

| analysis 提的"乱" | 本 spec 怎么解决 |
|-------------------|------------------|
| `UserMemory` = `ProjectMemory` 别名 | V0.1.x 后续清理（不在 V0.1.0 范围） |
| `/level` 是 echo | V0.1.x 后续清理（不在 V0.1.0 范围） |
| `SlashOutput` 5 变体只 1 个用 | V0.1.x 后续清理（不在 V0.1.0 范围） |
| `Concierge::placeholder()` 8+ 测试用 | **V0.1.0 处理**：删 + 改测试传 mock |
| `router.rs` 870 行 / `cache.rs` 595 行 | **V0.1.0 处理**：拆文件 |
| `RenderEngine` 单 impl 留位 | **V0.1.1 处理**：trait 改函数 |
| 4 家 provider 占位 | **V0.1.0 处理**：物理删 |
| 6 spec + 8 plan | **V0.1.2 处理**：合并归档 |
| LLM 真链路未跑通 | **V0.1.0 处理**：smoke test 3 次 |

---

*本 spec 待用户复审。复审通过后调 writing-plans skill 出 step-by-step 实施计划。*