# eflow 软件框架设计方案 v4.0

> **代号**：eflow — Efficient Flow（高效工作流）
> **主题愿景**：Efficient and Flexible Office（高效灵活的办公室）
>
> **版本**：v4.0 重构定稿
>
> **日期**：2026-06-15
>
> **技术语言**：Rust
>
> **状态**：架构审查完成，待里程碑拆解

---

## 0. 一句话定义

**eflow 是一款以 Rust 为核心的多层 Agent 协作框架，以"零阻塞对话"为第一设计原则，通过行业身份驱动的 SOP 调度、分层决策执行、智能上下文与记忆管理，让 AI 真正像一支训练有素的团队那样工作。**

---

## 1. 设计哲学

### 1.1 为什么叫 eflow

```
eflow = e(高效) + flow(工作流)
```

**核心理念：**
- **e（efficient）**：高效，用最少的步骤完成任务
- **flow（工作流）**：流畅，让 AI 像专业团队一样协作

**口号：** *One command to rule them all.* （一条命令搞定一切）

### 1.2 为什么是 Rust

Rust 是这个系统在技术层面的必然选择：

| 维度 | 选 Rust 的实际理由 |
|------|--------------------|
| **长驻稳定性** | 编译期消除内存泄漏和数据竞争，Agent 后台长跑不崩溃 |
| **多任务并发** | `tokio` 异步生态成熟，数十个 Subagent 并发执行零阻塞 |
| **单文件分发** | 编译为无依赖独立 exe，Windows 部署零成本 |
| **安全边界** | 所有危险操作在编译期 + 运行期双重拦截，不依赖运行时检查 |
| **性能** | 无 GC 停顿，事件总线吞吐量不受垃圾回收节拍影响 |
| **工具链** | Cargo 统一依赖、测试、打包，工程规范开箱即用 |

### 1.3 轻量化原则

- **核心层与扩展层明确分离** — 核心只含 Agent 调度、事件通道、上下文管理、记忆系统、原子工具；其余能力通过标准接口挂载
- **依赖数量硬控制** — v1.0 核心第三方 crate ≤ 15 个
- **单文件可运行** — 编译产物为独立可执行文件，无运行时依赖

---

## 2. 版本路线：渐进式激活

### 2.1 核心策略

**架构上保留完整能力规划，交付上按 feature flag 渐进式激活。** 不追求 v1.0 "完整版"一次性交付——那是 12-18 个月的工程，对开源产品来说风险过高。

### 2.2 能力地图

```
┌──────────────────────────────────────────────────────────┐
│                    eflow 能力地图                          │
│                                                          │
│  v1.0 内核 ──────────────────────────────────────────►    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │Concierge │  │Orchestr  │  │单Subagent│               │
│  │零阻塞对话│  │任务分解  │  │工具执行  │               │
│  └──────────┘  └──────────┘  └──────────┘               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │LLM Client│  │三层记忆  │  │事件通道  │               │
│  │多Provider│  │工作+持久 │  │broadcast │               │
│  └──────────┘  └──────────┘  └──────────┘               │
│                                                          │
│  v1.1 激活 ──────────────────────────────────────────►    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │多Subagent│  │磁盘缓存  │  │Profile   │               │
│  │并发池    │  │语义去重  │  │Skill系统 │               │
│  └──────────┘  └──────────┘  └──────────┘               │
│                                                          │
│  v1.2 激活 ──────────────────────────────────────────►    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │TUI交互   │  │过载保护  │  │沙箱执行  │               │
│  │ratatui   │  │          │  │          │               │
│  └──────────┘  └──────────┘  └──────────┘               │
│                                                          │
│  v2.0 激活 ──────────────────────────────────────────►    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐               │
│  │向量记忆  │  │GUI扩展   │  │数字签名  │               │
│  │语义检索  │  │egui/iced │  │Skill认证 │               │
│  └──────────┘  └──────────┘  └──────────┘               │
└──────────────────────────────────────────────────────────┘
```

### 2.3 优化重点的优先级

| 优先级 | 优化方向 | 依赖 | 归属版本 |
|--------|----------|------|----------|
| P0 | **对话上下文管理** | LLM Client | v1.0 内核 |
| P0 | **多层 Agent 互通工作流** | Context + Event | v1.0 内核 |
| P1 | **记忆系统** | Context 基础 | v1.0 内核 (三层) → v2.0 (向量) |
| P1 | **缓存命中率** | LLM Client + Context | v1.1 激活 |

---

## 3. 架构总览

### 3.1 四层架构

```
┌─────────────────────────────────────┐
│  交互层 (Interaction Layer)          │
│  ┌─────────┐    ┌──────────────────┐ │
│  │ CLI     │    │ TUI (v1.2)       │ │
│  │ (clap)  │    │ (ratatui)        │ │
│  └────┬────┘    └────────┬─────────┘ │
└───────┼──────────────────┼───────────┘
        │                  │
        ▼                  ▼
┌─────────────────────────────────────┐
│  应用编排层 (Application Layer)      │
│  ┌─────────────────────────────────┐ │
│  │  Concierge (管家)               │ │
│  │  - 独立 tokio task              │ │
│  │  - 零阻塞对话                   │ │
│  │  - 意图识别                     │ │
│  └──────────────┬──────────────────┘ │
│                 │                    │
│  ┌──────────────▼──────────────────┐ │
│  │  Orchestrator (协调者)          │ │
│  │  - 任务分解                     │ │
│  │  - Subagent 调度                │ │
│  │  - 结果聚合                     │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────┐
│  能力执行层 (Capability Layer)       │
│  ┌─────────────────────────────────┐ │
│  │  管线段：Decisioner → Executor   │ │
│  │                      → Feedbacker│ │
│  │  + 反馈回路（最多 3 次迭代）     │ │
│  │  共享上下文：Blackboard         │ │
│  └─────────────────────────────────┘ │
│  ┌─────────────────────────────────┐ │
│  │  Subagent Pool (子代理池)       │ │
│  │  - 角色/能力/权限隔离           │ │
│  │  - 独立生命周期                 │ │
│  │  - v1.0: 单 Subagent            │ │
│  │  - v1.1: 并发池                 │ │
│  └─────────────────────────────────┘ │
└─────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────┐
│  基础设施层 (Infrastructure Layer)   │
│  ┌────────┐ ┌────────┐ ┌────────┐  │
│  │LLMRoute│ │Context │ │Memory  │  │
│  │模型路由│ │上下文  │ │三层记忆│  │
│  └────────┘ └────────┘ └────────┘  │
│  ┌────────┐ ┌────────┐ ┌────────┐  │
│  │Event   │ │Profile │ │ToolReg │  │
│  │事件通道│ │角色系统│ │工具注册│  │
│  └────────┘ └────────┘ └────────┘  │
└─────────────────────────────────────┘
```

### 3.2 通信模式：同步调用 + 异步通知

跨层通信不是"全走事件总线"。区分两种通信模式：

| 通道 | 用途 | 实现 |
|------|------|------|
| **同步调用** | Orchestrator→Memory 读取记忆、Executor→LLM 调用、模块间的直接数据传递 | Rust trait 接口 + 函数调用 |
| **异步通知** | 任务开始/完成/失败通知 Concierge、风险升级弹确认框、系统关闭 | `tokio::broadcast` channel |

**核心设计决策**：类型安全的同步接口用于"做事"，事件通道用于"通知状态变化"。不把函数调用变成异步 RPC。

---

## 4. 零阻塞对话设计

### 4.1 核心矛盾与解决方案

**矛盾**：用户在任务执行期间需要随时对话，但传统 Agent 框架会让用户等待。

**方案**：Concierge 运行在独立 tokio task，与所有任务执行完全解耦。

```
用户 ──→ Concierge (独立 task)
              │
         发事件，不等结果
              │
         ┌────┴────┐
         │ 任务队列  │
         └────┬────┘
              │
         Orchestrator (异步)
              │
         能力执行层
```

**关键设计**：
1. Concierge 只负责接收用户输入、解析意图、发送任务
2. Concierge 不等待任何 LLM 响应或工具执行结果
3. 任务执行结果通过事件通道异步通知 Concierge
4. Concierge 通过监听事件更新会话状态

### 4.2 意图识别

Concierge 将用户输入分类为以下意图：

```rust
pub enum Intent {
    Chat { content: String },           // 纯对话，直接响应
    TaskDispatch { spec: TaskSpec },     // 派发给 Orchestrator
    TaskInterrupt { task_id: Uuid },     // 打断正在执行的任务
    TaskCancel { task_id: Uuid },        // 取消任务
    SkillQuery { keyword: String },      // 查询可用 Skill
    ProfileSwitch { industry: String },  // 切换行业 Profile
}
```

**v1.0 实现**：规则驱动（关键词 + 模式匹配）。意图识别不额外调用 LLM——避免"为了理解用户而先让用户等"。

---

## 5. 多层 Agent 互通工作流

### 5.1 管线段 + 反馈环

Decisioner / Executor / Feedbacker 不是平级模块——它们是**每个任务步骤的内部管线段**：

```
用户输入
  │
  ▼
┌─────────────────────────────────────────────────────┐
│ Concierge                                           │
│ 输入 → Intent + RawContext                          │
│ 产出：TaskSpec（不含执行细节，只说"要做什么"）        │
└────────────────────────┬────────────────────────────┘
                         │ TaskSpec
                         ▼
┌─────────────────────────────────────────────────────┐
│ Orchestrator                                        │
│ 输入 → TaskSpec                                     │
│ 产出：TaskPlan（分解为步骤序列，含依赖关系）           │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │          Per-Step Pipeline                   │    │
│  │                                             │    │
│  │  Decisioner ──► Executor ──► Feedbacker     │    │
│  │  (评估+路由)    (执行)       (质量评估)      │    │
│  │       ▲                        │            │    │
│  │       └────────────────────────┘            │    │
│  │         反馈回路（最多 3 次迭代）             │    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  产出：AggregatedResult                             │
└────────────────────────┬────────────────────────────┘
                         │ AggregatedResult
                         ▼
┌─────────────────────────────────────────────────────┐
│ Concierge                                           │
│ 输入 → AggregatedResult                             │
│ 产出 → 用户可读的最终回复                            │
└─────────────────────────────────────────────────────┘
```

### 5.2 三角色职责精确定义

| 角色 | 输入 | 输出 | 决策权 | 使用模型 |
|------|------|------|--------|----------|
| **Decisioner** | TaskStep + 当前 Blackboard | ExecutionPlan + RiskLevel + 模型选择 | 选哪个模型执行、风险等级、是否需要拆分子步骤 | 强模型 (Claude Opus / GPT-4) |
| **Executor** | ExecutionPlan + Blackboard | ActionResult + 工具调用记录 | 无决策权，只执行 | 轻量模型 (Claude Haiku / GPT-4o-mini) |
| **Feedbacker** | ActionResult + 预期产出 + Blackboard | QualityVerdict + 修正建议 | 判断结果是否达标、是否需要返工、是否需要升级风险 | 中等模型 (Claude Sonnet / GPT-4o) |

### 5.3 Blackboard：共享上下文

管道内流转的类型安全共享上下文，替代 `serde_json::Value` 做 payload：

```rust
/// 在 Orchestrator → Decisioner → Executor → Feedbacker 管道中流转
pub struct Blackboard {
    // --- 任务定义（Orchestrator 写入，下游只读）---
    pub task: TaskSpec,
    pub plan: TaskPlan,

    // --- 当前步骤（Decisioner 写入）---
    pub current_step: Option<TaskStep>,
    pub execution_plan: Option<ExecutionPlan>,
    pub risk_level: RiskLevel,

    // --- 执行历史（Executor 追加）---
    pub action_log: Vec<ActionRecord>,

    // --- 反馈历史（Feedbacker 追加）---
    pub feedback_log: Vec<FeedbackRecord>,
    pub retry_count: u8,

    // --- 跨步骤共享的临时状态 ---
    pub scratchpad: HashMap<String, serde_json::Value>,
}
```

**设计要点**：
- Blackboard 是值类型，每个阶段返回新版本（不可变更新），方便追踪和回滚
- `scratchpad` 是唯一的松散类型区域，给 Subagent 存放临时中间结果
- 管道结束时 Blackboard 被压缩为摘要存入记忆，**全量 Blackboard 不持久化**

### 5.4 反馈回路协议

Feedbacker 的输出有三种判决：

```rust
pub enum QualityVerdict {
    /// 通过，结果可用
    Pass { summary: String },
    /// 需要返工，携带修正指令
    Rework { reason: String, suggestion: String },
    /// 需要升级（风险等级不够、需人工介入等）
    Escalate { reason: String, new_risk: RiskLevel },
}
```

**回路流程**：

```
Feedbacker 判决
  │
  ├─ Pass ────► 步骤完成，进入下一步骤
  │
  ├─ Rework ──► retry_count < 3 ?
  │               ├─ Yes → 带着 suggestion 回到 Decisioner 重新规划
  │               └─ No  → 强制升级为 Escalate
  │
  └─ Escalate ─► L3 高危 → 暂停等用户确认
                  L2 及以下 → 记录告警，Executor 用更保守策略重试
```

### 5.5 上下文传递：逐层裁剪而非逐层膨胀

多层 Agent 的核心痛点：每层都往上加内容，管道末端上下文爆炸。**每层写摘要，下层带过滤**：

```
Concierge        产出 1KB 意图描述
    +
Orchestrator     产出 2KB 任务计划（含上一步意图摘要，非原文）
    +
Decisioner       产出 1KB 执行计划 + 风险评估（含上一步计划摘要）
    +
Executor         产出 2KB 执行结果 + 工具日志（含上一步计划摘要）
    +
Feedbacker       产出 0.5KB 质量判决（含执行结果摘要，丢弃工具日志）
    =
总计 ~6.5KB 有效上下文（vs 全文累积 30KB+）
```

**实现机制**：每个角色输出时调用 `ContextCompressor::summarize()`，将上游输入压缩为不超过 500 token 的摘要，附带原文引用指针（需要时可按指针回查）。

---

## 6. 对话上下文管理

### 6.1 问题定义

上下文管理回答三个问题：什么进上下文、满了怎么办、多 Agent 如何共享。

### 6.2 分层上下文模型

```
┌──────────────────────────────────────────────────┐
│  核心上下文 (Core Context)                        │
│  - 系统指令 (Profile)                             │
│  - 当前任务描述                                   │
│  - 当前 Blackboard 摘要                           │
│  - 最近 N 轮对话 (默认 N=5)                       │
│  → 每次 LLM 调用必然携带                          │
├──────────────────────────────────────────────────┤
│  参考上下文 (Reference Context)                   │
│  - 历史步骤的详细执行记录                         │
│  - 相关文件内容                                   │
│  - 记忆检索结果                                   │
│  → 携带摘要，原文按需通过引用指针回查              │
├──────────────────────────────────────────────────┤
│  丢弃 (Evicted)                                   │
│  - 过期工具调用日志                               │
│  - 失败尝试的详细堆栈                             │
│  - 与当前任务无关的历史对话                       │
│  → 不进入上下文，但保留在持久记忆中可被检索        │
└──────────────────────────────────────────────────┘
```

### 6.3 压缩策略：两级压缩

**L1 — 结构压缩（规则驱动，零 LLM 成本）**：
- 工具调用日志 → 只保留操作名 + 成功/失败 + 耗时
- 文件内容 → 只保留文件路径 + 行数 + 前 3 行
- 错误堆栈 → 只保留错误类型 + 第一行消息
- 适用场景：Executor 执行完成后的结果裁剪

**L2 — 语义压缩（调用轻量 LLM，有成本但质量高）**：
- 对话历史 → 压缩为"用户问了 X，系统做了 Y，结果是 Z"
- 多步骤执行 → 压缩为"经过 N 步完成了任务，关键决策点：..."
- 适用场景：会话过长、跨会话摘要、记忆持久化前

**触发时机**：

| 触发条件 | 压缩级别 | 说明 |
|----------|----------|------|
| 上下文 token 超过阈值 80% | L1 | 先裁剪冗余结构数据 |
| L1 压缩后仍超 80% | L2 | 对历史对话做语义压缩 |
| 会话结束持久化 | L2 | 生成会话摘要存入记忆 |
| 新任务开始 | L2 | 压缩上一个任务的完整上下文 |

### 6.4 压缩黄金法则：用户原话不可压缩

压缩最容易出问题的地方就是把用户原始意图"摘要"歪了。**不可压缩的保护区**：

```
绝不压缩（原文保留）：
  - 用户的原始输入
  - 系统对用户的直接回复
  - Profile 中的系统指令

L1 结构压缩（去格式，留事实）：
  - 工具调用日志：保留"做了什么+结果"，丢弃调试信息
  - 文件内容：路径+关键片段，丢弃全文

L2 语义压缩（可摘要，但有校验）：
  - 多轮对话历史 → 摘要 + 保留最近 N 轮原文
  - 执行过程 → 关键决策点，丢弃中间状态
```

**"度"的三层保障**：

1. **最近 N 轮不压缩**：和用户的最近对话（默认 5 轮）永远保留原文，压缩只针对更早的历史
2. **L2 语义压缩由 Feedbacker 校验**：如果压缩后的摘要与原意偏差，Feedbacker 标记返回修正或回退原文
3. **可回查是安全网**：即使压缩丢了细节，持久记忆保存了原文，LLM 发现摘要信息不够用时可通过引用指针自动回查原文——**压缩不是删除，是折叠**

### 6.5 引用指针

L1 压缩后丢弃的详细信息存下来，带上下文指针：

```rust
/// 上下文中的引用，替代原文
pub struct ContextRef {
    pub ref_id: Uuid,
    pub summary: String,              // 一行摘要，进入 LLM 上下文
    pub storage_key: String,          // 回查 key，指向 Memory 或磁盘缓存
    pub token_cost_if_included: u32,  // 原文的 token 成本标注
}
```

### 6.6 上下文生命周期

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ 会话开始  │───►│ 任务执行  │───►│ 任务完成  │───►│ 会话结束  │
│          │    │          │    │          │    │          │
│ 加载:    │    │ 每步骤:  │    │ 产出:    │    │ 产出:    │
│ Profile  │    │ Blackboard│   │ 任务摘要  │    │ 会话摘要  │
│ 会话记忆 │    │ 增量更新  │    │ 反馈记录  │    │ 关键决策  │
│ 历史摘要 │    │ 触发压缩  │    │ 写入项目记忆│    │ 写入项目记忆│
└──────────┘    └──────────┘    └──────────┘    └──────────┘
```

---

## 7. 记忆系统

### 7.1 三层记忆架构

原五层记忆（L0-L5）的边界在工程上不可区分。收敛为三层：

| 层级 | 存储 | 作用域 | 生命周期 | 存什么 | v1.0 |
|------|------|--------|----------|--------|------|
| **工作记忆** | 内存 (IndexMap) | 当前会话 | 会话结束销毁 | Blackboard摘要、当前任务状态、最近对话摘要 | ✅ |
| **项目记忆** | SQLite | 当前项目 | 持久，30天自动清理 | 任务历史、决策记录、文件操作日志、反馈记录 | ✅ |
| **用户记忆** | SQLite (v1.0) → 向量DB (v2.0) | 跨项目/全局 | 永久，手动清理 | 用户偏好、常用模式、跨项目经验 | v1.0基础 / v2.0增强 |

**删掉的层**：
- L0 "寄存器"：实现细节，不是架构层
- L5 "行业记忆"：概念与 Profile 系统重叠，合并入 Profile

### 7.2 记忆的读写时机

**写入（自动触发）**：

```
事件                              写入目标
────────────────────────────────────────────
每个步骤 Feedbacker 判决后    →   工作记忆（Blackboard 摘要）
每个任务完成后                →   项目记忆（任务摘要 + 关键决策 + 反馈结果）
会话结束时                    →   项目记忆（会话摘要 + 用户偏好变更）
用户明确说"记住这个"          →   用户记忆（标记为重要，永不过期）
连续 3 次相同操作模式         →   用户记忆（自动学习用户习惯）
```

**检索（按需触发）**：

```
场景                              检索目标
────────────────────────────────────────────
新任务开始                       →   项目记忆（查找相关历史任务）
用户问"上次/之前/那个..."        →   项目记忆 + 用户记忆（关键词 + 语义搜索）
Decisioner 做风险评估            →   项目记忆（查找类似操作的历史结果）
Concierge 识别意图               →   用户记忆（用户偏好和习惯）
上下文需要引用历史信息            →   工作记忆 → 项目记忆（逐层回退）
```

### 7.3 记忆与上下文的关系

```
记忆 = 存储层（仓库）          上下文 = 传输层（送到 LLM 面前的东西）

工作记忆 ──── 大部分直接进入 ────► 核心上下文（当前任务状态）
项目记忆 ──── 按需检索 ──────────► 参考上下文（通过引用指针）
用户记忆 ──── 按需检索 ──────────► 参考上下文（通过引用指针）
```

**关键约束**：记忆可以存很多，但永远不会把记忆全量灌进 LLM 上下文。

### 7.4 记忆管理器接口

```rust
/// 统一的记忆访问接口，屏蔽底层存储差异
pub trait MemoryManager: Send + Sync {
    /// 写入一条记忆
    fn remember(&self, entry: MemoryEntry) -> Result<Uuid>;
    /// 按关键词检索
    fn recall(&self, query: &str, scope: RecallScope, limit: u8) -> Result<Vec<MemoryEntry>>;
    /// 按时间范围检索
    fn recall_since(&self, since: SystemTime, scope: RecallScope) -> Result<Vec<MemoryEntry>>;
    /// 遗忘
    fn forget(&self, id: Uuid) -> Result<()>;
    /// 清理过期记忆
    fn cleanup(&self) -> Result<u32>;
    /// 导出当前会话摘要
    fn session_summary(&self) -> Result<String>;
}

pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,             // 压缩后的摘要
    pub raw_content: Option<String>, // 原文（可选，按引用回查）
    pub category: MemoryCategory,
    pub importance: Importance,      // 影响清理优先级
    pub created_at: SystemTime,
    pub last_accessed_at: SystemTime,
    pub ttl: Option<Duration>,
    pub tags: Vec<String>,
}

pub enum Importance {
    Low,       // TTL 到期自动清理
    Normal,    // 30 天清理
    High,      // 永不自动清理
    Pinned,    // 用户手动钉住
}
```

### 7.5 v2.0 向量记忆演进路径

v1.0 底层用 SQLite FTS5 全文搜索。v2.0 升级为向量检索时：
- **接口不变**：`MemoryManager` trait 实现替换
- **SQLite 保留**：作为结构化数据权威源，向量库作为语义索引层
- **渐进迁移**：新记忆双写，旧记忆按需批量向量化

---

## 8. 缓存系统

### 8.1 缓存场景分析

Agent 框架的 LLM 调用有四种场景，缓存价值完全不同：

| 调用场景 | 占比估算 | 重复概率 | 缓存策略 |
|----------|----------|----------|----------|
| 系统提示 + 对话 | ~30% | 高（Profile 固定，前缀重复） | **前缀缓存** |
| 工具调用定义 | ~10% | 极高（工具定义不变） | **静态缓存** |
| 任务执行的推理 | ~40% | 低（任务差异大） | **语义缓存** |
| 总结/反馈 | ~20% | 中（模式相似） | **语义缓存** |

### 8.2 三层缓存设计

```
┌─────────────────────────────────────────────────────────┐
│  L1: 前缀缓存 (Prefix Cache) — 零成本，最优先            │
│  - 利用 LLM API 原生前缀缓存 (Anthropic/OpenAI 均支持)   │
│  - System Prompt + Profile + 工具定义 → 固定前缀          │
│  - 命中率目标：> 80%（所有请求共享同一前缀部分）          │
│  - 实现：SDK 层面配置 cache_control breakpoint           │
├─────────────────────────────────────────────────────────┤
│  L2: 结构化键值缓存 — 任务结构匹配                        │
│  - Key: Hash(intent_type + task_signature + context_len) │
│  - 不是 hash 完整 messages，而是 hash 任务的结构特征     │
│  - 存储：内存 LRU (1000条) + SQLite 磁盘（7天）          │
│  - 命中率目标：> 30%                                     │
├─────────────────────────────────────────────────────────┤
│  L3: 语义缓存 — 相似任务复用 (v2.0)                     │
│  - Key: embedding 向量 + 余弦相似度 > 0.95               │
│  - 匹配的是"相似任务的历史结果"用作参考                  │
│  - v1.0 不做，v2.0 引入（依赖向量检索）                  │
└─────────────────────────────────────────────────────────┘
```

### 8.3 L2 结构化缓存 Key（核心设计）

为什么不用 SHA-256(完整 messages)？因为 messages 里有时戳、文件路径、具体变量名——任何一个字节不同就 miss。改用任务结构层面做 Key：

```rust
pub struct CacheKey {
    pub intent_type: IntentType,       // code_review | bug_fix | data_query | ...
    pub task_signature: String,        // 任务结构签名，如 "read_file_find_pattern"
    pub context_profile: ContextProfile,
    pub model: String,
}

pub struct ContextProfile {
    pub conversation_depth: u8,        // 会话长度范围
    pub file_count: u8,                // 涉及文件数
    pub risk_level: RiskLevel,
    pub profile: String,
}
```

**效果**：用户说"帮我审查 `src/main.rs` 的代码风格"和"帮我审查 `lib/parser.rs` 的代码风格"——Key 相同（`task_signature = code_review_style`），命中缓存。而"审查代码风格"和"审查安全漏洞"——Key 不同，正确区分。

### 8.4 缓存的值：结构化产物

缓存的值不是原始 LLM 响应文本，而是结构化产物：

```rust
pub enum CacheValue {
    Decision { plan: ExecutionPlan, risk: RiskLevel, model_choice: String },
    Execution { result: ActionResult, tool_calls: Vec<ToolCallSummary> },
    Feedback { verdict: QualityVerdict, confidence: f32 },
}
```

结构化缓存让跳过 LLM 调用成为可能——如果 Decisioner 返回相同的 ExecutionPlan，Executor 直接执行。

### 8.5 缓存有效性校验

L2 缓存命中后，结果给 Feedbacker 做一次快速校验（成本远低于完整执行）。如果 Feedbacker 判定不可用，标记该缓存条目并降级到正常 LLM 调用。

### 8.6 命中率度量

| 指标 | 计算方式 | 目标 |
|------|----------|------|
| 前缀缓存命中率 | 命中次数 / 总 LLM 请求数 | > 80% |
| L2 结构化缓存命中率 | 命中次数 / 总非前缀请求数 | > 30% |
| 缓存有效性（反向指标） | 命中但 Feedbacker 判定不可用 / 总命中 | < 5% |

---

## 9. Profile 与 Skill 系统

### 9.1 合并设计

Profile 和 Skill 在概念上重叠——一个"数据分析 Profile"本质上就是一组数据分析 Skill + 一条 System Prompt。合并为 Profile→Skill 树：

```rust
/// Profile = 角色身份，是 Skill 的容器
pub struct Profile {
    pub name: String,
    pub description: String,
    pub system_prompt: String,         // Jinja2 模板
    pub default_model: ModelTier,      // 默认使用的模型级别
    pub skills: Vec<String>,           // 挂载的 Skill 名称列表
    pub permission_boundary: PermissionSet,
}

/// Skill = 可复用的能力单元，属于某个 Profile
pub struct Skill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub prompt_template: String,       // Jinja2 模板路径
    pub required_tools: Vec<String>,   // 需要的工具
}
```

**加载关系**：Profile 加载 → 自动加载其 skills 列表 → 将 skills 的 prompt 模板注入 System Prompt → 完整 System Prompt 进入 L1 前缀缓存。

### 9.2 Skill 安全机制（v1.0 实用方案）

| 措施 | v1.0 实现 | v2.0 增强 |
|------|-----------|-----------|
| 防篡改 | SHA-256 校验和 | → 数字签名 |
| 权限控制 | 声明 + 运行时白名单校验 | → 沙箱隔离 |
| 版本管理 | 语义版本 + 兼容性声明 | → 锁定文件 |

**v1.0 Skill 加载流程**：

```
YAML 文件 → 校验 checksum → 解析结构 → 权限与 Profile 边界对照
                                                │
                                      ┌─────────┴──────────┐
                                      │ 越权 → 拒绝加载 + 告警 │
                                      │ 合法 → 注册到 Profile  │
                                      └────────────────────┘
```

---

## 10. 事件通道

### 10.1 设计决策：tokio::broadcast

不自建 Event Bus 框架。Rust 标准并发原语足够：

```rust
/// 系统事件通道
pub struct EventChannel {
    tx: broadcast::Sender<Event>,
}

impl EventChannel {
    pub fn publish(&self, event: Event) {
        let _ = self.tx.send(event); // 忽略无订阅者的错误
    }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
```

### 10.2 精简后的事件类型

只保留需要跨模块异步通知的 6 个核心事件：

```rust
pub enum Event {
    TaskStarted { task_id: Uuid, description: String },
    TaskCompleted { task_id: Uuid, summary: String },
    TaskFailed { task_id: Uuid, error: String },
    RiskEscalated { task_id: Uuid, from: RiskLevel, to: RiskLevel },
    UserInputRequired { prompt: String },
    SystemShutdown,
}
```

**删掉的 13 个原事件**：改为同步接口调用——`SubagentCreated`、`CacheHit`、`PlanGenerated` 等不需要跨层异步通知。

### 10.3 事件订阅关系

| 事件 | 发布者 | 订阅者 | 用途 |
|------|--------|--------|------|
| `TaskStarted` | Orchestrator | Concierge | 更新 UI 状态 |
| `TaskCompleted` | Orchestrator | Concierge | 通知用户结果 |
| `TaskFailed` | Orchestrator | Concierge | 通知用户失败 |
| `RiskEscalated` | Feedbacker | Concierge | 弹确认框 |
| `UserInputRequired` | 任意模块 | Concierge | 需要用户输入 |
| `SystemShutdown` | Concierge | 所有模块 | 优雅退出 |

---

## 11. LLM 集成架构

### 11.1 模型路由

```rust
/// 统一的 LLM 调用入口
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<Receiver<ChatChunk>>;
    fn supports_prefix_cache(&self) -> bool;
}

/// 模型层级 —— 对应三角色
pub enum ModelTier {
    Strong,    // Decisioner: Claude Opus / GPT-4
    Medium,    // Feedbacker: Claude Sonnet / GPT-4o
    Light,     // Executor:   Claude Haiku / GPT-4o-mini
}
```

### 11.2 路由策略与降级

| 角色 | 首选模型 | 降级模型 | 降级触发条件 |
|------|----------|----------|-------------|
| Decisioner | Claude Opus / GPT-4 | Claude Sonnet / GPT-4o | API 错误 ×3 或延迟 >10s |
| Executor | Claude Haiku / GPT-4o-mini | 同 tier 其他 provider | API 错误 ×2 |
| Feedbacker | Claude Sonnet / GPT-4o | Claude Haiku / GPT-4o-mini | API 错误 ×3 |

**v1.0 必须支持的 Provider**：Anthropic + OpenAI。Local (Ollama) 通过 feature flag 控制编译。

### 11.3 工具系统

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
    pub risk_level: RiskLevel,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}
```

**v1.0 内置工具**：`read_file`、`write_file`、`execute_command`、`search_code`、`web_fetch`。不超过 10 个。

---

## 12. 过载保护

### 12.1 核心指标修正

Agent 框架的瓶颈是 LLM API，不是本地 CPU。以 API 状态为核心降级指标：

| 负载级别 | 触发条件 | 动作 |
|----------|----------|------|
| 正常 | API 延迟正常，token 余量充足 | 全功能 |
| 轻度降级 | 连续 5 次 API 限流 (429) | Executor 降级到更轻模型 |
| 中度降级 | 连续 10 次 API 限流 | 暂停非关键 Subagent，排队处理 |
| 重度降级 | Token 配额耗尽或 API 不可用 | 仅保留 Concierge 对话，通知用户 |

CPU 监控保留为次级指标（本地推理场景——Ollama 等）。

---

## 13. Subagent 体系

### 13.1 Subagent 定义

```rust
pub struct Subagent {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    pub capabilities: Vec<Capability>,
    pub permission: PermissionSet,
    pub lifecycle: SubagentLifecycle,
    pub context: Context,
}
```

### 13.2 生命周期

1. **创建** — 由 Orchestrator 根据任务需求创建
2. **执行** — 接收任务，调用对应工具
3. **反馈** — 将结果通过 Event Channel 返回
4. **销毁** — 任务完成，释放资源

### 13.3 版本演进

| 版本 | 能力 |
|------|------|
| v1.0 | 单 Subagent，串行执行步骤 |
| v1.1 | 多 Subagent 并发池，独立生命周期 |
| v2.0 | 跨项目 Subagent 复用，预热池 |

---

## 14. 交互层设计

### 14.1 InteractionLayer trait

```rust
/// 交互层抽象接口，核心层通过 trait 与交互层交互
pub trait InteractionLayer: Send + Sync {
    fn receive(&self, input: String) -> Result<Event>;
    fn subscribe(&self) -> Receiver<Event>;
    fn shutdown(&self);
}
```

### 14.2 CLI 交互（v1.0）

- 框架：`clap`（命令行解析）
- 模式：交互式对话
- 快捷键：Enter 发送 / Ctrl+C 中断 / Ctrl+Q 退出 / ↑↓ 滚动历史

### 14.3 TUI 交互（v1.2）

- 渲染：`ratatui`
- 终端 IO：`crossterm`（Windows 全终端兼容）
- 刷新：tokio 异步推送

### 14.4 GUI 扩展（v2.0 预留）

- 框架：`egui` 或 `iced`
- 实现 `InteractionLayer` trait 即可接入，核心层无感知

---

## 15. 配置文件设计

```yaml
# eflow.yaml
core:
  language: zh-CN
  timezone: Asia/Shanghai

llm:
  providers:
    anthropic:
      api_key: ${ANTHROPIC_API_KEY}
      default_model: claude-sonnet-4-6
    openai:
      api_key: ${OPENAI_API_KEY}
      default_model: gpt-4o
  routing:
    strong: anthropic     # Decisioner
    medium: anthropic     # Feedbacker
    light: openai         # Executor
  cache:
    l1_enabled: true      # 前缀缓存
    l2_enabled: false     # 结构化缓存 (v1.1)
    l2_ttl_days: 7

memory:
  working_memory_limit: 1000
  project_db_path: ./data/project.db
  user_db_path: ./data/user.db
  cleanup_interval_hours: 24
  auto_learn: false       # 自动学习用户习惯 (v2.0)

security:
  risk_threshold: L2       # 超过此级别需人工确认
  allowed_paths:
    - ~/projects
    - ~/documents

profiles:
  default: developer
  available:
    - developer
    - data_analyst
    - writer
```

---

## 16. 目录结构

```
eflow/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── CHANGELOG.md
├── docs/
│   ├── spec/                    # 架构规格
│   ├── plans/                   # 开发计划
│   └── review/                  # 审查报告
├── src/
│   ├── main.rs                  # 入口
│   ├── lib.rs                   # 库入口 + feature flags
│   ├── interaction/             # 交互层
│   │   ├── mod.rs               # InteractionLayer trait
│   │   └── cli.rs               # CLI 实现
│   ├── application/             # 编排层
│   │   ├── mod.rs
│   │   ├── concierge.rs         # 零阻塞对话入口
│   │   └── orchestrator.rs      # 任务分解 + Subagent 调度
│   ├── capability/              # 能力层（三角色管线段）
│   │   ├── mod.rs
│   │   ├── decisioner.rs        # 风险评估 + 模型路由
│   │   ├── executor.rs          # 步骤执行
│   │   ├── feedbacker.rs        # 质量判决 + 反馈回路
│   │   ├── blackboard.rs        # 共享上下文
│   │   ├── subagent.rs          # Subagent 定义 + 生命周期
│   │   └── tools/               # 工具实现
│   │       ├── mod.rs
│   │       ├── registry.rs
│   │       ├── file.rs
│   │       ├── command.rs
│   │       └── search.rs
│   ├── infrastructure/          # 基础设施
│   │   ├── mod.rs
│   │   ├── llm/                 # LLM 集成
│   │   │   ├── mod.rs
│   │   │   ├── router.rs
│   │   │   ├── anthropic.rs
│   │   │   ├── openai.rs
│   │   │   └── cache.rs
│   │   ├── memory/              # 三层记忆
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs
│   │   │   ├── working.rs
│   │   │   ├── project.rs
│   │   │   └── user.rs
│   │   ├── context/             # 上下文管理
│   │   │   ├── mod.rs
│   │   │   ├── compressor.rs
│   │   │   └── reference.rs
│   │   ├── profile/             # Profile + Skill
│   │   │   ├── mod.rs
│   │   │   ├── loader.rs
│   │   │   └── skill.rs
│   │   └── event.rs             # 事件通道
│   └── common/                  # 公共类型
│       ├── mod.rs
│       ├── error.rs
│       └── types.rs
├── profiles/                    # 内置 Profiles
│   ├── developer.yaml
│   ├── data_analyst.yaml
│   └── writer.yaml
├── templates/                   # Prompt 模板 (Jinja2)
│   ├── code_review.jinja2
│   └── summary.jinja2
├── tests/                       # 集成测试
├── benches/                     # 性能测试
└── data/                        # 运行时数据
    ├── cache/                   # 磁盘缓存
    └── logs/                    # 日志
```

---

## 17. 风险等级体系

| 等级 | 名称 | 示例 | 执行方式 |
|------|------|------|----------|
| L0 | 只读 | 读取文件、查询数据库 | 自动执行 |
| L1 | 文件写入 | 创建文件、修改配置 | 自动执行 + 安全检查 |
| L2 | 系统命令 | 安装软件、重启服务 | 沙箱隔离执行 |
| L3 | 高危操作 | 删除数据、修改系统设置 | 人工确认 |

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    L0,  // 只读操作
    L1,  // 文件写入
    L2,  // 系统命令
    L3,  // 高危操作（需人工确认）
}
```

---

## 18. 质量目标

| 指标 | 目标值 | 测量方式 |
|------|--------|----------|
| 单元测试覆盖率 | 核心模块 ≥ 70% | `cargo tarpaulin` |
| 代码检查 | 零告警 | `cargo clippy` |
| 格式规范 | 无格式问题 | `cargo fmt` |
| 并发安全 | 零数据竞争 | `cargo test` 竞态模式 |
| 二进制体积 | < 50MB | `--release` + 符号剥离 |
| 启动时间 | < 1s | 基准测试 |
| L1 前缀缓存命中率 | > 80% | 内置监控指标 |
| L2 结构化缓存命中率 | > 30% | 内置监控指标 |
| LLM 调用成功率（含重试） | > 99% | 内置监控指标 |

---

## 19. 架构决策记录（ADR）

### ADR-0001：零阻塞对话设计

**状态**：已接受

**背景**：用户在任务执行期间无法正常对话是所有 Agent 框架的核心痛点。

**决策**：Concierge 运行在独立 tokio task，与所有任务执行完全解耦。对话入口永不等待任何 LLM 响应或工具执行。Concierge 只发事件，不等结果。

**代价**：Concierge 无法直接知道任务完成状态，需要监听事件回调更新会话状态。

### ADR-0002：三角色分层决策

**状态**：已接受

**背景**：所有任务都用同一个大模型处理，成本高且效率低。

**决策**：Decisioner 用强模型评估风险并路由，Executor 用轻量模型执行，Feedbacker 用中等模型总结并形成反馈回路。三角色构成管线段而非平级模块。

**代价**：增加了系统复杂度，需要维护三个模型的调用逻辑。

### ADR-0003：同步调用 + 异步通知分离

**状态**：已接受

**背景**：原设计规定所有跨层通信走 Event Bus，但单进程 Rust 应用中这增加了不必要的序列化开销和类型安全损失。

**决策**：同层模块和数据传递使用类型安全的同步接口，跨层状态通知使用 tokio::broadcast 事件通道。事件类型从 19 个精简为 6 个核心事件。

**代价**：需要明确区分"做事"和"通知"的边界，设计上需要更多思考。

### ADR-0004：单二进制设计

**状态**：已接受

**决策**：收敛为单二进制 `eflow`，通过参数区分模式，TUI 作为主要交互方式。

### ADR-0005：无 Web 后台设计

**状态**：已接受

**决策**：专注于 CLI/TUI 体验，不内置远程监控功能。远程监控作为 v2.0 可选插件。

### ADR-0006：GUI 预留接口

**状态**：已接受

**决策**：定义 `InteractionLayer` trait，核心层通过 trait 与交互层交互，未来只需实现该 trait 即可接入 GUI。

### ADR-0007：Profile 与 Skill 合并

**状态**：已接受

**背景**：原设计中 Profile 和 Skill 是两个独立系统，但"数据分析 Profile"本质上就是一组数据分析 Skill + 一条 System Prompt。

**决策**：合并为 Profile→Skill 树。Profile 是容器（角色身份 + System Prompt），Skill 是挂载的能力单元（prompt 模板 + 工具组合）。

### ADR-0008：五层记忆收敛为三层

**状态**：已接受

**背景**：原五层记忆的边界在实际工程中不可区分。L0 是实现细节，L5 与 Profile 重叠，L1/L2 边界模糊。

**决策**：收敛为工作记忆（会话级，内存）、项目记忆（项目级，SQLite）、用户记忆（跨项目，SQLite→v2.0 向量DB）。

### ADR-0009：缓存 Key 用任务结构而非完整输入

**状态**：已接受

**背景**：SHA-256(完整 messages) 的精确匹配在 Agent 场景命中率趋近于零——messages 中任何字节不同就 miss。

**决策**：L2 缓存 Key 基于 `intent_type + task_signature + context_profile` 的结构特征匹配，而非完整输入哈希。缓存有效性由 Feedbacker 校验。

---

## 20. 国际化（i18n）

### 20.1 范围

eflow v1.0 支持中英双语，**默认简体中文**，可切换英文显示。

| 语言 | 标识 | 状态 |
|------|------|------|
| 简体中文 | `zh-CN` | 默认 |
| English | `en-US` | 可选 |

### 20.2 设计目标

- **默认中文** — 开箱即用面向国内用户
- **可切换英文** — 满足国际化协作和开源贡献者习惯
- **切换不需重编译** — 通过运行时配置生效
- **翻译范围全覆盖** — 系统硬编码字符串、错误消息、状态输出、Profile/Skill 模板中的固定提示语均双语
- **用户原话不翻译** — 与铁律 5「用户原话不可压缩」对应：用户输入原文与系统直接回复原文保持原样

### 20.3 实现机制

使用 `rust-i18n` crate 提供的编译期宏 `t!()`：

- 资源文件位于 `locales/` 目录，按语言代码命名（`zh-CN.yml`、`en-US.yml`）
- 启动时按优先级确定 locale（见 20.5）
- 运行时通过 `rust_i18n::i18n::set_locale()` 切换
- **fallback 链**：`zh-CN` 缺失时回退到 `en-US`，避免单点翻译遗漏导致界面出现混合语言

### 20.4 翻译资源组织

```
eflow/
├── locales/
│   ├── zh-CN.yml        # 简体中文（默认）
│   └── en-US.yml        # English
```

YAML 文件结构（按 key 组织）：

```yaml
# zh-CN.yml
_system_prompt: |
  你是 eflow，一个高效的多层 Agent 协作框架的入口。
err_profile_not_found: "未找到 profile: %{name}"
err_provider_not_found: "未找到 provider: %{name}"
err_http: "HTTP 错误: %{msg}"
err_json_parse: "JSON 解析错误: %{msg}"
err_config_load: "加载配置失败: %{msg}"
err_config_parse: "解析配置失败: %{msg}"
err_read_profiles_dir: "读取 profiles 目录失败: %{msg}"
err_read_entry: "读取目录条目失败: %{msg}"
err_read_file: "读取文件 %{path} 失败: %{msg}"
err_parse_file: "解析文件 %{path} 失败: %{msg}"
err_no_provider: "未找到 tier %{tier} 对应的 provider"
err_rate_limited: "%{provider} 限流，已重试 %{count} 次，放弃"
status_profile_loaded: "已加载 profile '%{name}' (checksum: %{checksum})"
ctx_file_summary: "文件 %{path} (%{lines}行, %{bytes}字节)"
ctx_error_summary: "错误: %{msg}"
```

```yaml
# en-US.yml
_system_prompt: |
  You are eflow, the entry point of an efficient multi-layer Agent collaboration framework.
err_profile_not_found: "Profile not found: %{name}"
err_provider_not_found: "Provider not found: %{name}"
err_http: "HTTP error: %{msg}"
err_json_parse: "JSON parse error: %{msg}"
err_config_load: "Failed to load config: %{msg}"
err_config_parse: "Failed to parse config: %{msg}"
err_read_profiles_dir: "Failed to read profiles dir: %{msg}"
err_read_entry: "Failed to read directory entry: %{msg}"
err_read_file: "Failed to read file %{path}: %{msg}"
err_parse_file: "Failed to parse file %{path}: %{msg}"
err_no_provider: "No provider for tier %{tier}"
err_rate_limited: "%{provider} rate-limited after %{count} retries, giving up"
status_profile_loaded: "Loaded profile '%{name}' (checksum: %{checksum})"
ctx_file_summary: "file %{path} (%{lines} lines, %{bytes} bytes)"
ctx_error_summary: "error: %{msg}"
```

**资源键命名约定**：
- `err_*`：错误消息（在 `EflowError` 的 `Display` 实现里使用）
- `status_*`：状态/进度消息（tracing 日志、CLI 横幅）
- `ctx_*`：上下文压缩器输出（用于 LLM 上下文的格式化字符串）
- 下划线开头（如 `_system_prompt`）保留为元键，不直接翻译

### 20.5 切换优先级

启动时 locale 按以下优先级确定（高优先级覆盖低优先级）：

| 优先级 | 来源 | 行为 |
|--------|------|------|
| 1（最高） | CLI 启动参数 `--lang=zh-CN` | 启动时立即生效（**M13 实施**） |
| 2 | `eflow.yaml` 中 `core.language` | 启动时读取（**M2 已有字段**，M7.5 接入 i18n） |
| 3（最低） | 默认值 `zh-CN` | 内置常量 |

**API 形式**：

```rust
// src/infrastructure/locale.rs
pub fn init_from_config(config_locale: Option<&str>) -> Locale {
    let locale = config_locale
        .filter(|s| is_supported(s))
        .unwrap_or(DEFAULT_LOCALE);
    rust_i18n::i18n::set_locale(locale);
    Locale::from_str(locale)
}

pub const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en-US"];
pub const DEFAULT_LOCALE: &str = "zh-CN";
```

### 20.6 v1.0 范围之外

- 运行时切换语言（不重启）— v1.0 启动时确定后保持不变
- 自动检测系统语言 — 需在 CLI 层读取 OS locale，v1.0 不做
- 其他语言（日语、韩语等）— v2.0 按需扩展
- Locale 相关的日期/数字/货币格式 — chrono 自身能力足够，v1.0 不引入完整本地化栈
- Profile/Skill 描述（`description` 字段）的运行时翻译 — YAML 是用户自己写的中文/英文，不翻

### 20.7 ADR-0010：双语方案

**状态**：已接受

**背景**：eflow 面向国内用户，开发者与开源贡献者中也有英文使用者。两类用户都需要看清系统消息、错误提示、Profile 描述等所有用户可见字符串。

**决策**：
- v1.0 支持 `zh-CN`（默认）和 `en-US` 两种
- 用 `rust-i18n` crate（成熟方案、零运行时开销、YAML 资源易维护）
- 翻译范围覆盖系统硬编码字符串 + 错误消息 + Profile/Skill 模板中的固定提示语
- 切换优先级：CLI 参数 > 配置文件 > 默认值

**代价**：
- 增加 1 个第三方依赖
- 双语资源文件需随功能更新同步维护
- 启动时需读取 locale（额外 ~1ms 开销，可忽略）

---

## 21. 铁律（不可违反）

1. **测试先行** — 没有测试的代码不得合并
2. **最小实现** — 不要过度工程化，先做能用的，再做完美的
3. **单一职责** — 每个模块只做一件事
4. **风险门控不可绕过** — L3 任务绕过门控直接执行属于 P0 bug
5. **用户原话不可压缩** — 上下文压缩永远保留用户原始输入和直接回复
6. **记忆不灌入上下文** — 记忆检索结果经压缩后以引用指针进入上下文，不全文携带
7. **沙箱路径不可越权** — 文件操作越出白名单路径直接 panic + 审计记录
8. **缓存 Key 含模型版本** — 模型升级后旧缓存自动失效
9. **零硬编码配置** — 所有可变参数在配置文件中定义
10. **cargo clippy 零告警** — CI 中 clippy 警告等同 build 失败

---

*文档状态：v4.0 重构定稿，待里程碑拆解*
