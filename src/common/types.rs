use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ========== 风险等级 ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    L0, // 只读操作 — 自动执行
    L1, // 文件写入 — 自动执行 + 安全检查
    L2, // 系统命令 — 沙箱隔离执行
    L3, // 高危操作 — 人工确认
}

// ========== 意图 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Chat { content: String },
    TaskDispatch { spec: TaskSpec },
    TaskInterrupt { task_id: Uuid },
    TaskCancel { task_id: Uuid },
    SkillQuery { keyword: String },
    ProfileSwitch { industry: String },
}

// ========== 任务规范 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: Uuid,
    pub description: String,
    pub risk_level: RiskLevel,
    pub priority: TaskPriority,
    pub steps: Vec<TaskStep>,
    pub timeout_secs: u64,
    pub retry_policy: RetryPolicy,
}

impl TaskSpec {
    pub fn new(description: String, risk_level: RiskLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            description,
            risk_level,
            priority: TaskPriority::Normal,
            steps: vec![],
            timeout_secs: 300,
            retry_policy: RetryPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub action: String,
    pub tool: String,
    pub params: serde_json::Value,
    pub expected_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u8,
    pub backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_ms: 1000,
        }
    }
}

// ========== 任务计划 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: Uuid,
    pub steps: Vec<PlannedStep>,
    pub estimated_steps: u8,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub order: u8,
    pub action: String,
    pub tool: String,
    pub params: serde_json::Value,
    pub depends_on: Option<u8>, // 依赖的前置步骤序号
}

// ========== 执行计划 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub step: PlannedStep,
    pub model_tier: ModelTier,
    pub risk_level: RiskLevel,
    pub sub_steps: Vec<TaskStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    Strong, // Decisioner: Claude Opus / GPT-4
    Medium, // Feedbacker: Claude Sonnet / GPT-4o
    Light,  // Executor:   Claude Haiku / GPT-4o-mini
}

// ========== 执行结果 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub output: String,
    pub tool_calls: Vec<ToolCallSummary>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub tool_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub summary: String,
}

// ========== 操作记录 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub tool: String,
    pub success: bool,
    pub summary: String,
}

// ========== 反馈 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub timestamp: DateTime<Utc>,
    pub verdict: QualityVerdict,
    pub retry_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityVerdict {
    Pass { summary: String },
    Rework { reason: String, suggestion: String },
    Escalate { reason: String, new_risk: RiskLevel },
}

// ========== 角色/能力 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    FileAssistant,
    CodeAssistant,
    DataAnalyst,
    Generalist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    ReadFile,
    WriteFile,
    ExecuteCommand,
    SearchCode,
    WebFetch,
    LlmReasoning,
}

// ========== 权限 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSet {
    pub allowed_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub max_file_size_bytes: u64,
    pub network_enabled: bool,
}

impl Default for PermissionSet {
    fn default() -> Self {
        Self {
            allowed_paths: vec![],
            allowed_commands: vec![],
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
            network_enabled: false,
        }
    }
}

// ========== 上下文引用 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRef {
    pub ref_id: Uuid,
    pub summary: String,
    pub storage_key: String,
    pub token_cost_if_included: u32,
}

// ========== 意图类型（缓存 Key 用）==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentType {
    CodeReview,
    BugFix,
    DataQuery,
    FileRead,
    FileWrite,
    CommandExecute,
    WebFetch,
    Chat,
    Unknown,
}

// ========== 记忆类型 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryCategory {
    TaskResult,
    Decision,
    Feedback,
    UserPreference,
    LearnedPattern,
    ManualNote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Importance {
    Low,
    Normal,
    High,
    Pinned,
}
