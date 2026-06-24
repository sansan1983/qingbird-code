# eflow 现状盘点与打磨建议

> 日期:2026-06-24
> 范围:`/home/ubuntu/eflow`(v1.4 spec D 已合并)
> 数据:86 个 Rust 文件 / 11,234 行代码 / 174 tests / 4 门禁全过
> 状态:**代码本身是好的,乱的不是质量,而是"为未来预留的代码 + spec 文档膨胀"**

---

## 一句话诊断

eflow 的"乱"集中在三处:

1. **为未发生的未来预留的代码**(trait 只有一个 impl / "v1.5+ 可加" / "v2.0 升级" 注释)
2. **占位/未接通的实现**(`/level` 是 echo / `SlashOutput` 5 个变体只用 1 个 / `UserMemory` 是 `ProjectMemory` 别名)
3. **规划文档比代码重**(6 份 spec + 8 份 plan + 多份 ADR)

代码本身、测试覆盖、4 门禁(build / test / clippy / fmt)都健康。

---

## 现状盘点(真 vs 虚)

| 类别 | 数量/内容 | 状态 |
|------|----------|------|
| **真接通,端到端可跑** | TUI / CLI / `eflow --execute` / `eflow session start` / `eflow init` 向导 / 6 个斜杠命令 | ✅ 实际可用 |
| **真接通,只跑 1 个实例** | `RenderEngine`(1 个 impl)/ `RenderBackend`(1 个 impl)/ `Wizard`(1 个 init 用例) | ⚠️ 抽象做了,单实例 |
| **写了接口但没接通** | `/level` / `SlashOutput::ReloadRouter\|Shutdown\|OpenSubView` / `Memory::user` = `project` 别名 | 🟡 摆设 |
| **占位/测试专用** | `Concierge::placeholder()` / `LlmRouter::placeholder()`(8+ 测试用)/ `t!("..._placeholder")` | 🟡 测试味 |
| **为未来写** | "v1.5+ 可加 HighContrastEngine" / "v2.0 升级到向量搜索" / "v2.0 GUI 套壳" | 🔴 死代码 |
| **规划/文档** | 6 spec + 8 plan + 多份 ADR | ⚠️ 比代码还重 |

---

## 6 个具体的"乱"的地方

### 1. `UserMemory` = `ProjectMemory` 别名

`src/infrastructure/memory/user.rs:2`

```rust
// v1.0: User memory shares SQLite implementation with project memory.
// v2.0: upgrade to vector search.
pub use super::project::ProjectMemory as UserMemory;
```

注释自己说"v2.0 才升级",现在就是一个 re-export。3 层记忆(WORKING / PROJECT / USER)在配置里要给 2 个 db path,实际背后同一个实现。**直接删 user.rs,把 CompositeMemory 改成 2 层**。

### 2. `/level` 命令是纯 echo

`src/interaction/slash/builtin/level.rs`

```rust
// v1.3.3+ 阶段占位 —— v1.3.3 spec C 实施未接通派发路径
async fn execute(&self, args: SlashArgs, _ctx: &mut CommandContext) -> Result<SlashOutput> {
    let level = args.first().cloned().unwrap_or_default();
    Ok(SlashOutput::Text(t!("status_level_changed", level = level).into_owned()))
}
```

CLAUDE.md 自己写了:**"`/level` 命令从'假切档'改'占位提示'——承认 v1.3.3 spec C 实施未接通"**。stub 还在生产路径跑。

### 3. `SlashOutput` 5 个变体,只用 1 个

`src/application/concierge.rs:168-176`

```rust
SlashOutput::ReloadRouter => t!("err_cmd_failed", msg = "ReloadRouter").to_string(),
SlashOutput::Shutdown => t!("err_cmd_failed", msg = "Shutdown").to_string(),
SlashOutput::OpenSubView(_) => t!("err_subview_render_failed", ...).to_string(),
```

`enum SlashOutput { Text, NoOp, ReloadRouter, Shutdown, OpenSubView }` — 5 个变体,实际只有 `Text` 被返回,其他 3 个走 i18n 错误兜底。**简化成 `enum SlashOutput { Text(String) }` 或者直接返 `String`**。

### 4. `Concierge::placeholder()` 在 8+ 测试里用

`src/application/concierge.rs:235`

```rust
/// **非测试代码不应调用**——用 `Concierge::new`。
#[doc(hidden)]
impl Concierge {
    pub fn placeholder() -> Self { ... }
}
```

注释自己说"非测试代码不应调用",但 `#[doc(hidden)]` 是唯一挡板。`LlmRouter::placeholder()` 同款。**承认"测试和生产用的是两个 Concierge",说明 Concierge 的依赖图太重,测试组装成本高**。

- 短期:`#[cfg(test)]` 真的挡住
- 长期:Concierge 改成 DI,测试传 mock

### 5. `router.rs` 870 行 + `cache.rs` 595 行

`src/infrastructure/llm/`

LLM 路由层两个超大文件:routing 表 / rate limiter / L1 cache / L2 cache / retry / placeholder 全混一起。`cache_key_for_step` 这种 helper 被 `decisioner.rs` / `executor.rs` / `feedbacker.rs` 三个角色 import。

**拆法**:
- `router.rs` → 只剩 tier routing + retry
- `cache.rs` → 拆 `l1_cache.rs`(in-memory LRU)+ `l2_cache.rs`(SQLite)
- 抽 `lifecycle.rs` 装 placeholder / fallback / from_config

### 6. 渲染管线"为未来 v1.5+ 设计"

`src/interaction/render/render_engine.rs:4`

```rust
//! 未来 v1.5+ 可加 HighContrastEngine / DarkEngine 等替代 impl。
```

刚合并的 v1.4 spec D,4 个文件,31 新测试。**问题**:`RenderEngine` trait 只有一个 impl(`DefaultRenderEngine`),已经在为第二个 impl 留位 — 这跟 qingbird 的"13 接口"模式一样。

---

## 其它零散问题(也该修)

| 位置 | 问题 |
|------|------|
| `main.rs:250` | `initial_cache_hit_rate = "0/0"` 写死,注释说"v1.3 接 L2 cache stats" — L2 实际已写好,应该从 router 拿真值 |
| `src/interaction/slash/builtin/model.rs:78` | `/model` 选完没真切换,只 echo(类似 `/level`) |
| `src/infrastructure/profile/loader.rs:56,79` | "v1.0: 简单校验;v2.0: 数字签名" / "v1.0: 基础校验,v2.0: 沙箱隔离" — 5 年后才用得上的代码应该删而不是注释 |
| `~/.eflow/providers/` 目录不存在 | v1.3 LLM 抽象允许"放 yaml 文件就加载 provider",实际上**所有人都用 env var fallback**。要么删抽象,要么真用 |

---

## 打磨路径(按风险从低到高)

### P0:纯删(1-2 小时,零风险)

| # | 动作 | 预计减 |
|---|------|------|
| 1 | 删 `Memory::user`,3 层改 2 层 | -30 行 + 1 文件 |
| 2 | 删 `SlashOutput` 的 3 个变体,简化 | -30 行 |
| 3 | `Concierge::placeholder()` / `LlmRouter::placeholder()` 加 `#[cfg(test)]` | -5 行,挡住误用 |
| 4 | 删 `/level` 斜杠命令 + 相关 i18n key | -50 行 + 1 文件 |
| 5 | 删 3 个 SlashOutput 错误 i18n key | -10 行 |

**总计:-125 行左右,减 1-2 个文件依赖,功能不变**(本来就没人用)。172 tests 全过。

### P1:小重构(半天,低风险)

- 拆 `router.rs` (870 → 3 个文件)
- 拆 `cache.rs` (595 → 2 个文件)
- 删 "v2.0"/"v1.5+" 注释(不留,删那行代码或简化注释)

零功能改动,纯结构调整。

### P2:中等重构(1-2 天,有取舍)

- `RenderEngine` trait → 改回 struct + 静态方法(如果决定不留多 impl)
- `RenderBackend` trait → 同上
- `/model` 选完真切换(如果决定保留 /model)
- `main.rs` 的 `initial_cache_hit_rate` 写死改成从 router 拿

### P3:文档/可选回退

- 6 spec 合并成 1 份"当前架构说明"
- 8 plan 合并成 1 份"未来计划"或直接删旧的
- v1.4 渲染管线:见下方判断

---

## 关于"回退"

### ❌ 不建议回退:v1.3.x 整套(spec A/B1/B2)

LLM 抽象 + 6 个斜杠命令 + 协议契约 — eflow 的**实际能力**,跑得起来。回退等于砍功能。

### ⚠️ 看情况:v1.4 渲染管线(刚合并)

4 个文件 + 31 新测试,有真价值(ViewModel 路径让 wizard/SelectList 零 ratatui 依赖)。

- **保留**:ViewModel 5 个 struct / DrawCommand enum / `state_to_vm()` 方法 / `execute_draw_commands` helper
- **可回退**:`RenderEngine` trait + `DefaultRenderEngine` → 改成 `fn render_step(&StepViewModel) -> Vec<DrawCommand>` 一个函数

判断标准:**1 年内有没有可能加第二个 RenderEngine 实现?**
- 没有(常见情况)→ 留 trait 是浪费,改回函数
- 有(暗色主题 / 高对比度 / 打印模式)→ 留

### ❌ 不建议回退:v1.3.3 spec C(已经回退过了)

PR #21 收尾时已经把 513 行 workflow 抽象整套删了 — **这一步是已经发生的事**。CLAUDE.md 自己写了"承认 spec C 实施未接通"。

**这件事本身是个信号**:"spec 写满 → 实施不通 → 大块删回退" 这个模式对你反复出现(qingbird 7 接口错了,eflow v1.3.3 撤回,现在又在 v1.4 加 trait)。后面遇到新模块时,需要先写第二个实例再抽 trait。

---

## 最核心建议(给你,不要重复犯)

eflow 的代码不是"乱",是 **spec 文档和"未来预留"占了一半的认知负担**。

**功能跑得起来**(`eflow --execute "task"` 真能跑)
**测试 174 个**(真在测东西)
**门禁 4 个都过**
**"乱"是文档/未来预留/未接通的占位** 的混乱,**不是代码本身**

**所以**:

1. **先做 P0 的纯删**(1-2 小时,零风险)— 把"摆设有但没接"的砍掉
2. **再合并 spec 文档** — 6 份 spec 合并成 1 份"当前架构说明",8 份 plan 合并成 1 份"未来计划"或直接删旧的
3. **P1/P2 看精力做**
4. **v1.4 渲染管线** — 保留 ViewModel + DrawCommand,把 trait 改函数(如果你想最简化)

---

## ADR(可以写进 `docs/adr/0018-eflow-polish.md`)

```markdown
# ADR-0018: eflow 现状盘点与打磨原则

## 当时怎么想的
v1.0 → v1.4 一路设计,4 门禁全过,174 tests pass。看起来"健康"。

## 后来发现(2026-06-24 复盘)
- 11,234 行代码,约 20% 是"为未来预留"的代码
- 6 spec + 8 plan,文档比代码重
- /level / SlashOutput::ReloadRouter|Shutdown|OpenSubView / UserMemory 这些是摆设
- Concierge::placeholder() / LlmRouter::placeholder() 在测试里被 8+ 次用
- 注释里到处是"v1.5+ / v2.0 / 未来 / 占位"

## 现在怎么办
- 砍掉所有"未接通的摆设"(P0 1-2 小时)
- 合并 spec 文档(6→1,8→1 或直接归档)
- 渲染管线 trait 改函数(单实例场景)
- 新增模块:有第二个实例再抽 trait

## 教训
- "门禁全过"≠"系统健康"
- "tests pass"≠"功能完整"(测试可以测摆设)
- "为未来设计"在反复犯(spec C 撤回 / v1.4 trait 化)
- 真正健康的标志:**所有接通的代码都有人用,所有有人用的代码都接通**
```

---

**下一步选择**:
- A. 直接动 P0(5 个纯删,1 个 PR)
- B. 先看 P0 列表,标哪些能删哪些要留,我再动
- C. 先合 spec 文档,不碰代码
- D. 只想再讨论,不动
