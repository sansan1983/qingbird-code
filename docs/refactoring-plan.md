# qingbird v0.2.1 重构方案

> 目标：核心循环从"120 行内联 loop"重构为"状态机 + hook + 显式 IO"。
> 不改外部接口，不改现有测试，不改 Provider/工具/模型层。

## 一、当前问题

`ReactLoop::run()` 120 行做了 3 件不该在一起的事：

1. **状态判断**（下一步该干啥）
2. **IO 调用**（LLM HTTP 请求、工具执行）
3. **安全检测**（死循环 + Nudge）

混在一起导致不能暂停、不能独立测试状态转换、每加功能就要改那 120 行。

## 二、目标架构

```
改前:                             改后:
src/react_loop/                   src/react_loop/
  mod.rs    ← 120 行循环体          mod.rs    ← re-export + run() 编排
  types.rs  ← 类型                  types.rs  ← 新增 Step/Hook/HookAction
                                    loop.rs   ← 新增: 状态机决策函数
                                    hooks.rs  ← 新增: 死循环+Nudge 包装
```

### 新增类型

```rust
/// 每一步：驱动方根据这个决定做什么
pub enum Step {
    CallLlm { messages: Vec<Message>, tool_schemas: Vec<serde_json::Value> },
    CallTools { calls: Vec<ToolCall> },
    Done(AgentResult),
}

/// Hook 处理结果
pub enum HookAction {
    Proceed,          // 继续
    Nudge(String),    // 注入引导消息后继续
    Halt(String),     // 立即终止
}

/// Agent hook trait — 安全机制通过这个接口注入
pub trait AgentHook: Send {
    fn on_llm_response(&mut self, response: &ChatResponse, state: &LoopState) -> HookAction;
    fn on_tool_results(&mut self, results: &[ToolResult], state: &LoopState) -> HookAction;
}
```

### 新增决策函数

```rust
impl ReactLoop {
    /// 纯逻辑：决定下一步做什么
    fn decide_next_step(&self, state: &mut LoopState, messages: &[Message],
        tool_schemas: &[serde_json::Value]) -> Step;

    /// 纯逻辑：处理 LLM 响应，更新 state 和 messages
    fn process_llm_response(&self, state: &mut LoopState, response: &ChatResponse,
        messages: &mut Vec<Message>) -> Result<Step, EflowError>;

    /// 纯逻辑：工具执行后的收尾
    fn after_tools(&self, state: &mut LoopState, messages: &[Message]) -> Step;
}
```

## 三、数据流

```
run() 内部:
loop {
    step = decide_next_step(state, messages, tool_schemas)
    match step {
        CallLlm  → IO: build_request → http_client.send → parse_response
                 → Hook: on_llm_response()
                 → process_llm_response(state, response, messages)
        CallTools → IO: execute_tools_parallel/serial
                  → Hook: on_tool_results()
                  → after_tools(state, messages)
        Done(r)  → return Ok(r)
    }
}
```

关键变化：IO 在 match 里集中、显式，状态转换在纯函数里。

## 四、不改的部分

| 模块 | 原因 |
|------|------|
| `doom_loop.rs` | 算法不变，只改调用方式 |
| `nudge.rs` | 消息生成不变，只改调用位置 |
| `subagent.rs` | 只是 ReactLoop 包装器，接口不变 |
| 全部 infra crate | Provider/HTTP/Config 不动 |
| 全部 tools crate | 4 个工具 + 注册表不动 |
| 全部 models crate | 类型不动 |
| `ReactLoop::run()` 签名 | 外部不变 |
| 全部现有测试 | 照跑 |

## 五、改动清单

| 操作 | 文件 | 说明 |
|------|------|------|
| 新建 | `react_loop/types.rs` | Step/HookAction/AgentHook 定义 |
| 新建 | `react_loop/loop.rs` | 状态机：decide_next_step / process_llm_response / after_tools |
| 新建 | `react_loop/hooks.rs` | AgentHooks: 包装 doom_detector + nudge_system |
| 重写 | `react_loop/mod.rs` | run() 用新结构编排，保留外部签名 |
| 删除 | `react_loop/types.rs` 原 dead enum | TurnResult / ExecutionStrategy |

## 六、版本路线图

```
v0.2.1  核心循环重构    状态机 + hook，功能零变化
v0.2.2  多轮对话       --interactive 保留历史
v0.2.3  thinking 配置   从 config 读取，不硬编码
v0.2.4  Provider 路由   LlmConfig.active 接线
v0.3.0  LlmClient trait  Provider + HTTP 合并
```
