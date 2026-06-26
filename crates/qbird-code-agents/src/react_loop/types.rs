use qbird_code_models::ToolCall;

/// 每轮迭代后的控制流结果
#[derive(Debug, Clone)]
pub enum TurnResult {
    /// 继续下一轮迭代
    Continue,
    /// 需要执行工具调用
    ToolCalls { tool_calls: Vec<ToolCall> },
    /// 任务完成
    Complete {
        content: String,
        status: Option<String>,
    },
    /// 达到最大迭代次数
    MaxIterations,
    /// 用户中断
    Interrupted,
}

/// Agent 最终结果
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub content: String,
    pub messages: Vec<qbird_code_models::Message>,
    pub usage: qbird_code_models::UsageStats,
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

/// Loop 内部控制流信号
#[allow(dead_code)]
pub(super) enum LoopAction {
    Continue,
    Return(Result<AgentResult, qbird_code_models::EflowError>),
}

/// 工具执行策略
#[derive(Debug)]
pub enum ExecutionStrategy {
    ParallelSubagents,
    BatchedReadonly,
    Sequential,
}

/// 只读工具名称集合（用于并行批处理判定）
///
/// Note: `glob` and `list_dir` are planned for V0.1.1 and do not yet exist
/// in the tools crate. They are listed here as forward-looking entries.
pub(super) static READ_ONLY_TOOLS: &[&str] = &["read_file", "search_code", "glob", "list_dir"];

/// ReAct 循环配置
#[derive(Debug, Clone)]
pub struct ReactLoopConfig {
    pub max_iterations: usize,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    /// 连续只读轮次上限（超过后触发 nudge）
    pub max_consecutive_reads: usize,
    /// 上下文 token 压缩阈值（百分比，如 0.8 表示 80% 触发压缩）
    pub compaction_threshold: f32,
}

impl Default for ReactLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            model: "deepseek-v4-pro".into(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            max_consecutive_reads: 5,
            compaction_threshold: 0.8,
        }
    }
}
