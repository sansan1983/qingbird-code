# qingbird 架构设计 v0.2

> 当前版本 V0.2.0，从 eflow v1.x 重构后的 5-crate workspace 架构。

## 一、项目定位

qingbird 是一个**单二进制 LLM Agent 框架**，核心能力是：
- 接收用户自然语言指令
- 通过 ReAct 循环 + 工具调用 完成编码辅助类任务
- 支持多种 LLM Provider（DeepSeek / Ollama / OpenAI / Anthropic）

---

## 二、5-Crate 依赖关系

```
┌─────────────────────────────────────────────┐
│ qbird-code (binary)                          │
│ CLI 入口: --execute / --interactive          │
├──────────────────┬──────────────────────────┤
│ qbird-code-agents│ qbird-code-tools          │
│ ReAct 循环       │ 4 内置工具                │
│ 死循环检测       │ (读/写/搜索/执行命令)     │
│ Nudge 机制       │ ToolRegistry              │
│ Subagent         │                           │
├──────────────────┴──────────────────────────┤
│ qbird-code-infra                             │
│ 4 LLM Provider + HTTP 客户端 (重试+退避)     │
│ 配置加载 (qingbird.yaml)                     │
├─────────────────────────────────────────────┤
│ qbird-code-models                            │
│ Message, EflowError, RiskLevel, TaskSpec...  │
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
| `config.rs` | 加载 qingbird.yaml，DeepseekConfig/OllamaConfig/OpenaiConfig/AnthropicConfig |
| `http_client.rs` | 统一 HTTP 客户端，支持 timeout + 指数退避重试 |
| `providers/` | 4 个 Provider，均实现 `Provider` trait |
| `providers/mod.rs` | `Provider` trait 定义（build_request_body / parse_response / build_headers / endpoint） |
| `event.rs` | 事件通道（tokio::broadcast），旧架构遗留，当前未被 binary 使用 |
| `env.rs` | 环境变量展开（${VAR} → 值） |
| `locale.rs` | i18n 辅助 |

Provider trait 核心接口:
```
build_request_body(&self, messages, config) → Value
parse_response(&self, body) → ChatResponse
build_headers(&self) → HashMap
endpoint(&self) → String
```

### 4.3 qbird-code-tools

工具系统。

| 工具 | 风险等级 | 执行策略 |
|------|---------|---------|
| `read_file` | L0 | 并行 |
| `search_code` | L0 | 并行 |
| `write_file` | L1 | 串行 |
| `execute_command` | L2 | 串行 |

`ToolRegistry` 提供注册/查找/执行能力，内置 L3 风险拦截。

### 4.4 qbird-code-agents

Agent 循环层。

| 模块 | 职责 |
|------|------|
| `react_loop/mod.rs` | 主循环：LLM 调用 → 工具执行 → 循环/返回 |
| `react_loop/types.rs` | LoopState / AgentResult / ReactLoopConfig |
| `doom_loop.rs` | 死循环检测：基于工具调用指纹循环检测 |
| `nudge.rs` | 引导注入：连续只读/无工具/接近上限/完成检查 |
| `subagent.rs` | Subagent 包装器（ReactLoop 的简单封装） |

### 4.5 qbird-code (binary)

二进制入口，负责装配所有组件。

| 路径 | 职责 |
|------|------|
| `main.rs` | CLI 解析 + 组件初始化 + --execute/--interactive 路由 |

---

## 五、当前架构的局限性

### 5.1 功能缺失（对标旧 eflow v1.x 已实现的功能）

| 旧 eflow 功能 | 当前状态 | 说明 |
|---------------|---------|------|
| **多 Provider 动态路由** | ❌ | 硬编码 DeepSeekProvider，`LlmConfig.active` 字段未使用 |
| **TUI 交互** | ❌ | ratatui 已从依赖删除，只有 CLI REPL |
| **多轮对话记忆** | ❌ | --interactive 每轮创建新 messages，不保留历史 |
| **项目/用户记忆** (SQLite) | ❌ | memory 系统未移植 |
| **Profile/Skill 系统** | ❌ | 旧的 ProfileRegistry 已删除 |
| **上下文压缩** | ❌ | compaction_threshold 字段存在但未实现 |
| **Subagent 池** | ❌ | Subagent 模块存在但 binary 未使用 |
| **事件总线** | ⚠️ | event.rs 存在但 binary 未接入 |
| **配置向导/Wizard** | ❌ | 旧的 wizard 系统已删除 |
| **Headless 模式** (NDJSON) | ❌ | 旧的 cli/start.rs 已删除 |
| **风险升级 / L3 人工确认** | ⚠️ | ToolRegistry 有 L3 拦截，但无人工确认路径 |

### 5.2 代码质量问题

见上一轮分析报告：
1. 工具调用 JSON 解析重复
2. `thinking_enabled` 硬编码
3. 工具错误信息丢失（中文硬编码 + 类型擦除）
4. 3 个死 enum (`RecoveryAction`, `TurnResult`, `ExecutionStrategy`)

---

## 六、迭代路线图建议

根据当前"核心循环可用，周边能力缺失"的状态，建议按以下顺序迭代：

### Phase A — 核心循环硬化（P0/P1 代码质量问题）

```
1. 工具调用解析去重        → 提取 helper: tool_calls_from_response()
2. 删除 3 个死 enum         → 清理 types.rs
3. thinking_enabled 接入配置 → ReactLoopConfig + RequestConfig 联动
4. 工具错误保留原始 Error   → 不用中文 "错误:" 前缀
```

### Phase B — 多轮对话记忆

```
5. --interactive 保留历史   → messages 跨轮传递
6. 简单上下文窗口管理        → 超出 N 轮时丢弃最早的 user/assistant 对
```

### Phase C — 多 Provider 路由

```
7. 用 LlmConfig.active 选择 Provider
8. 支持 --provider 命令行参数覆盖
9. Provider 级联降级（主 provider 失败 → 备用）
```

### Phase D — Agent 协作

```
10. Subagent 真正接入 binary → 复杂任务拆分为子任务
11. Subagent 池复用         → 多 subagent 并发执行
12. 结果汇总 → 协调器
```

### Phase E — 记忆层

```
13. 项目记忆 (SQLite FTS5)
14. 用户偏好记忆
15. 上下文压缩 (摘要/截断)
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

根据上面分析，当前最适合立即动手的方向：

**推荐优先做**: Phase A（核心循环清理）→ Phase B（多轮记忆）

原因：
- Phase A 全是小改动，且解决的是代码质量问题，风险低
- Phase B 是功能补齐，--interactive 无历史太影响体验，用户感知最强
- 做完这两步后，项目有了"可用的交互式 Agent"基础能力
