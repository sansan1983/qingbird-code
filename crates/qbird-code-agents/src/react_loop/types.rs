use qbird_code_models::{Message, ToolCall, UsageStats};

// ===== 公开类型 =====

/// Agent 最终结果
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub content: String,
    pub messages: Vec<Message>,
    pub usage: UsageStats,
}

/// 循环中跨迭代保持的可变状态
pub struct LoopState {
    pub iteration: usize,
    pub consecutive_no_tool_calls: usize,
    pub consecutive_reads: usize,
    pub read_nudge_sent: bool,
    pub completion_nudge_sent: bool,
    pub no_tool_nudge_sent: bool,
}

impl LoopState {
    pub fn new() -> Self {
        Self {
            iteration: 0,
            consecutive_no_tool_calls: 0,
            consecutive_reads: 0,
            read_nudge_sent: false,
            completion_nudge_sent: false,
            no_tool_nudge_sent: false,
        }
    }
}

impl Default for LoopState {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态机步骤：驱动方根据这个决定做什么
#[derive(Debug, Clone)]
pub enum Step {
    /// 调用 LLM
    CallLlm,
    /// 执行工具
    CallTools { tool_calls: Vec<ToolCall> },
    /// 完成
    Done(AgentResult),
}

/// Hook 对事件的响应
pub enum HookAction {
    /// 继续
    Proceed,
    /// 注入 nudge 消息后继续
    Nudge(String),
    /// 立即终止
    Halt(String),
}

/// Agent hook trait — 安全机制通过这个接口注入
pub trait AgentHook: Send {
    /// LLM 响应后、状态更新前调用
    fn on_llm_response(&mut self, state: &LoopState) -> HookAction;
    /// 工具执行结果处理后调用
    fn on_tool_results(&mut self, state: &LoopState) -> HookAction;
}

/// ReAct 循环配置
#[derive(Debug, Clone)]
pub struct ReactLoopConfig {
    pub max_iterations: usize,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    /// 连续只读轮次上限（超过后触发 nudge）
    pub max_consecutive_reads: usize,
}

impl Default for ReactLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            model: "deepseek-v4-pro".into(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            max_consecutive_reads: 5,
        }
    }
}

// ===== 内部常量 =====

/// 只读工具名称集合（用于并行批处理判定 + 连续只读检测）
pub(super) static READ_ONLY_TOOLS: &[&str] = &["read_file", "search_code", "glob", "list_dir"];
