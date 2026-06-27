# qingbird 架构设计 v0.3

> 当前目标版本 V0.3.0（v0.2.18 清理 + v0.2.19 接线 + v0.3.0 打磨）。从 eflow v1.x 重构后的 5-crate workspace 架构。

## 〇、v0.3.0 状态表

| 模块 | 状态 | 说明 |
|------|------|------|
| 5 LLM Provider 路由 | ✅ v0.2.4 | DeepSeek (OpenAI/Anthropic) / Ollama / OpenAI / Anthropic |
| 多轮对话记忆 | ✅ v0.2.15 | SQLite + FTS5，会话可重载 |
| 上下文 Token 预算 | ✅ v0.2.15 | ContextManager 预算化窗口 + 自动检查点 |
| Subagent 并发池 | ✅ v0.2.16 | SubagentPool + execute_parallel |
| i18n 中英双语 | ✅ v0.2.12 | rust_i18n 编译期宏，所有用户字符串走 `t!()` |
| 错误 i18n 全量 | ✅ v0.2.18 | `EflowError::user_message()` + 全量审计 |
| SDD proposal 状态机 | ✅ v0.2.18 | `/sdd confirm` 真实接通 `hard_gate_blocked` |
| LLM 流式输出 | ✅ v0.2.19 | 全 5 provider，`StreamFormat` + 共享 SSE parser |
| RuntimeOverrides | ✅ v0.2.19 | provider/model/temperature 瞬时态隔离 yaml |
| Config 校验 | ✅ v0.2.19 | `EflowConfig::validate()` 6 条规则聚合输出 |
| Cache L1 / fast_model / risk_threshold | ✅ v0.2.19 | yaml 字段真接通 |
| Session 生命周期 | ✅ v0.3.0 | `/session delete` / `/session rename` / LRU 50 |
| Profile 系统 | ✅ v0.3.0 | 用户级 yaml，CLI + `/profile` + 工具白名单 |
| `edit` 工具 + 撤销栈 | ✅ v0.3.0 | `similar` diff 输出 + `/undo` 20 上限 |
| 成本追踪 | ✅ v0.3.0 | per-provider cost + USD/RMB 显示 |
| EventChannel 接入 | 🔜 v0.4+ | 占位清理完成，待事件总线方向 C |

## 一、项目定位

qingbird 是一个**单二进制 LLM Agent 框架**，核心能力是：
- 接收用户自然语言指令
- 通过 ReAct 循环 + 工具调用 完成编码辅助类任务
- 支持多种 LLM Provider（DeepSeek / Ollama / OpenAI / Anthropic）
- 提供 Profile 系统（用户级 yaml 覆盖 ReactLoop 配置 + 工具白名单 + system prompt）

---

## 二、5-Crate 依赖关系

```
┌─────────────────────────────────────────────┐
│ qbird-code (binary)                          │
│ CLI 入口: --execute / --interactive          │
│           --provider / --model / --temperature│
│           --lang / --profile / --stream      │
├──────────────────┬──────────────────────────┤
│ qbird-code-agents│ qbird-code-tools          │
│ ReAct 循环       │ 8 内置工具                │
│ 死循环检测       │ 读/写/搜索/命令/glob/    │
│ Nudge 机制       │ list_dir/web_fetch/edit   │
│ Subagent 池      │ ToolRegistry (白名单)     │
│                  │ UndoStack                 │
├──────────────────┴──────────────────────────┤
│ qbird-code-infra                             │
│ 5 LLM Provider (流式) + HTTP 客户端 (重试+退避)│
│ 配置加载 (qingbird.yaml) + Profile loader     │
│ RuntimeOverrides + StreamFormat + SSE parser │
├─────────────────────────────────────────────┤
│ qbird-code-models                            │
│ Message, EflowError, RiskLevel,              │
│ PermissionSet, Role, Capability,             │
│ MemoryCategory, Importance, RetryPolicy      │
└─────────────────────────────────────────────┘
```

**严格依赖方向**: 下层禁止引用上层。
- models ← infra ← {agents, tools} ← binary

---

## 三、核心数据流

### 3.1 启动流程 (main.rs)

```
CLI 参数解析 (clap)
    │
    ▼
加载配置 qingbird.yaml (find_config → load_config)
    │
    ▼
创建 HttpLlmClient      ─── HTTP 重试+退避
创建 Provider (目前硬编码 DeepSeekProvider)
    │
    ▼
创建 ToolRegistry
  ├── ReadFileTool
  ├── WriteFileTool
  ├── ExecuteCommandTool
  └── SearchCodeTool
    │
    ▼
工具定义 → 转 JSON Schema
    │
    ├── --execute "prompt"
    │      └── ReactLoop::run()
    │
    └── --interactive
           └── REPL 循环 → ReactLoop::run() 每轮
```

### 3.2 ReAct 循环 (核心循环)

```
                    ┌──────────────────────────┐
                    │ 1. iteration += 1         │
                    │ 2. check_safety(上限?)    │──── 超限 → 返回 nudge 文本
                    │ 3. check_nudges(...)      │──── 连续只读/无工具/接近上限
                    └──────────┬───────────────┘
                               ▼
                    ┌──────────────────────────┐
                    │ 4. 构造 LLM 请求           │
                    │   messages + tool_schemas │
                    │   → build_request_body()   │
                    └──────────┬───────────────┘
                               ▼
                    ┌──────────────────────────┐
                    │ 5. http_client.send()     │──── 带重试+退避
                    │ 6. provider.parse_response│
                    └──────────┬───────────────┘
                               ▼
                    ┌──────────────────────────┐
                    │ 7. 判断响应类型            │
                    │                           │
                    │ 有 tool_calls?             │
                    │   ├── 全部只读? → 并行执行 │
                    │   └── 有写入? → 串行执行   │
                    │         │                 │
                    │         ▼                 │
                    │   死循环检测               │──── ForceStop → 返回
                    │   (DoomLoopDetector)       │
                    │         │                 │
                    │         ▼                 │
                    │   工具执行结果             │
                    │   → tool_result 消息       │
                    │         │                 │
                    │         └──→ 回到步骤 1    │
                    │                           │
                    │ 无 tool_calls?             │
                    │   finish_reason="stop"?    │
                    │   ├── 有写入? → Nudge      │
                    │   └── 无问题 → 返回结果    │
                    └──────────────────────────┘
```

### 3.3 交互模式数据流

```
--interactive 模式:
  用户输入 "分析这个文件"
       │
       ▼
  每次都创建全新的 messages:
    [system_prompt, user("分析这个文件")]
       │
       ▼
  ReactLoop::run() → 返回 AgentResult.content
       │
       ▼
  打印结果 → 等待下一条输入

  ⚠ 注意: 交互模式不保留历史消息。
    每轮对话都是独立的。
```

---

## 四、各模块职责

### 4.1 qbird-code-models

纯数据类型 crate，无逻辑。

| 类型 | 用途 |
|------|------|
| `Message` / `MessageRole` | 统一消息格式，兼容 OpenAI/Anthropic |
| `ToolCall` / `ToolCallFunction` | 工具调用描述 |
| `EflowError` | 全项目错误枚举 |
| `RiskLevel` | L0-L3 风险等级 |
| `UsageStats` | Token 用量统计 |

### 4.2 qbird-code-infra

基础设施层。

| 模块 | 职责 |
|------|------|
| `config.rs` | 加载 qingbird.yaml；DeepseekConfig / OllamaConfig / OpenaiConfig / AnthropicConfig；`EflowConfig::validate()` 6 条规则 |
| `http_client.rs` | 统一 HTTP 客户端，支持 timeout + 指数退避重试（`RetryPolicy` 驱动） |
| `providers/` | 5 个 Provider，均实现 `Provider` trait，支持流式 |
| `providers/mod.rs` | `Provider` trait 定义（build_request_body / parse_response / build_headers / endpoint / stream_format / stream） |
| `stream_format.rs` | `StreamFormat` enum + `StreamEvent` enum |
| `stream_parser.rs` | OpenAI / Anthropic SSE parser |
| `runtime_overrides.rs` | `RuntimeOverrides { provider, model, temperature }` 瞬时态 |
| `profile.rs` | `Profile` loader + merge_into，扫描 `data_dir()/qingbird/profiles/*.yaml` |
| `event.rs` | 事件通道（tokio::broadcast），占位待 v0.4+ 接入 |
| `env.rs` | 环境变量展开（${VAR} → 值） |
| `locale.rs` | i18n 辅助；`init(locale: &str)` 显式接受 locale |
| `memory/` | SQLite + FTS5 记忆层（MemoryManager / ContextManager / SessionStore） |

Provider trait 核心接口:
```
build_request_body(&self, messages, config) → Value
parse_response(&self, body) → ChatResponse
build_headers(&self) → HashMap
endpoint(&self) → String
stream_format(&self) → StreamFormat
stream(&self, req) → Stream<StreamEvent>
```

### 4.3 qbird-code-tools

工具系统（v0.3.0 共 8 个）。

| 工具 | 风险等级 | 执行策略 |
|------|---------|---------|
| `read_file` | L0 | 并行 |
| `search_code` | L0 | 并行 |
| `glob` | L0 | 并行 |
| `list_dir` | L0 | 并行 |
| `web_fetch` | L0 | 并行 |
| `write_file` | L1 | 串行 |
| `edit` | L1 | 串行（精确匹配 + UndoStack） |
| `execute_command` | L2 | 串行 |

`ToolRegistry` 提供注册/查找/执行能力，内置 L3 风险拦截 + Profile 驱动的 `allowed_tools` 白名单。

### 4.4 qbird-code-agents

Agent 循环层。

| 模块 | 职责 |
|------|------|
| `react_loop/mod.rs` | 主循环：LLM 调用 → 工具执行 → 循环/返回；真接 ContextManager / MemoryManager / Streaming |
| `react_loop/types.rs` | LoopState / AgentResult / ReactLoopConfig / PermissionSet / Capability |
| `react_loop/hooks.rs` | AgentHook trait 扩展：risk gate / profile 白名单 |
| `doom_loop.rs` | 死循环检测：基于工具调用指纹循环检测 |
| `nudge.rs` | 引导注入：连续只读/无工具/接近上限/完成检查 |
| `subagent.rs` | Subagent 包装器（ReactLoop 的简单封装） |
| `subagent_pool.rs` | SubagentPool mpsc + N worker 并发池 |

### 4.5 qbird-code (binary)

二进制入口，负责装配所有组件。

| 路径 | 职责 |
|------|------|
| `main.rs` | CLI 解析 + 组件初始化 + --execute/--interactive 路由 |

---

## 五、v0.3.0 已消除的局限

| 旧局限 | 解决版本 | 方案 |
|--------|---------|------|
| 多 Provider 动态路由 | v0.2.4 | `LlmConfig.active` 字段接通；`RuntimeOverrides` 隔离瞬时态 |
| 多轮对话记忆 | v0.2.2 | messages 跨轮传递；v0.2.15 升级到 SQLite + FTS5 |
| 项目/用户记忆 | v0.2.15 | `MemoryManager` + `ContextManager` + `SessionStore` |
| Profile/Skill 系统 | v0.3.0 | `Profile` struct + 用户级 yaml + `/profile` 斜杠命令 |
| 上下文压缩 / 预算 | v0.2.15 | `ContextManager` 预算化窗口 + 自动检查点 |
| Subagent 池 | v0.2.16 | `SubagentPool` mpsc + N worker |
| 流式 LLM 输出 | v0.2.19 | `StreamFormat` + 共享 SSE parser + 全 5 provider |
| Config 校验 | v0.2.19 | `EflowConfig::validate()` 6 条规则聚合 |
| 工具调用解析重复 | v0.2.17 | 抽取 `tool_calls_from_response()` helper |
| `thinking_enabled` 硬编码 | v0.2.3 | `ReactLoopConfig` 字段 + yaml 接线 |
| 工具错误信息丢失 | v0.2.18 | `EflowError::user_message()` + i18n 审计 |
| 死 enum | v0.2.18 | 删 9 个 write-only 类型（Intent/TaskSpec/PlannedStep/FeedbackRecord/QualityVerdict/ActionResult/TaskStep/TaskPlan/IntentType） |
| 事件总线 | v0.4+ | 占位清理，方向 C 排期 |

---

## 六、v0.4+ 路线图

v0.3.0 完成后，剩余未消除的局限及建议方向：

### Phase F — 事件总线方向 C

```
16. EventChannel 接入 binary → subagent 事件流 → 实时 UI
17. 事件 schema 冻结（v0.4 spec B2）
18. NDJSON stdout 契约（headless 模式）
```

### Phase G — Subagent 方向 C 续

```
19. Subagent capability 路由
20. 任务分解协调器（Orchestrator 替代品）
```

### Phase H — 新能力方向 D

```
21. 配置向导（首次启动检测无配置时触发）
22. TUI 重新接入（ratatui / 独立进程套壳）
23. 长期记忆压缩（LLM-driven summarization）
```

---

## 七、组件关系时序图（正常完成路径）

```
User          CLI          ReactLoop           LLM               Tools
 │             │              │                 │                  │
 │ --execute   │              │                 │                  │
 │────────────>│              │                 │                  │
 │             │──run()──────>│                 │                  │
 │             │              │──build_request──>│                  │
 │             │              │<──response──────│                  │
 │             │              │                 │                  │
 │             │              │  tool_calls?    │                  │
 │             │              │  └──read_file───│─────────────────>│
 │             │              │<──result───────────────────────────│
 │             │              │                 │                  │
 │             │              │──build_request──>│                  │
 │             │              │<──response──────│                  │
 │             │              │  no tool_calls  │                  │
 │             │              │  stop + content │                  │
 │             │<──Ok(content)─│                 │                  │
 │<──print─────│              │                 │                  │
```

---

## 八、改进入口建议

v0.3.0 已交付"日常编码助手"基线。下一步建议方向：

**v0.4 优先**: 事件总线方向 C（Phase F）—— 详见设计文档 `docs/superpowers/specs/2026-06-27-qingbird-v0.3-cleanup-and-daily-usable-design.md` §六。

**长期**: TUI 重新接入（Phase H）—— 待事件总线稳定后，ratatui 套壳可基于稳定的事件契约。
