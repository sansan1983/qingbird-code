//! `SubagentProfile` 数据模型 + 内置 profile 字典。
//!
//! 一个 profile = 一个子 agent 角色定义（系统提示词前置段 + 工具策略 +
//! 描述 + 默认工具集）。LLM 通过 `delegate_task` 工具按 profile 名字
//! 派发子任务。

use serde::{Deserialize, Serialize};

/// 工具策略：决定子 agent 可用工具集。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolPolicy {
    /// 只读工具集
    ReadOnly,
    /// 继承父 agent 的完整工具集
    Inherit,
}

/// Subagent 模式（预留字段；v0.3.1 只用 Subagent）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentMode {
    /// 子 agent：独立 ReAct 循环实例，独立 session
    Subagent,
    /// 主 agent（预留：v0.4+ 启动多 persona 时使用）
    Primary,
}

/// Subagent profile 配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentProfile {
    pub name: String,
    pub mode: SubagentMode,
    pub tool_policy: ToolPolicy,
    pub prompt_preamble: String,
    pub description: String,
    pub default_tools: Vec<String>,
    pub max_iterations: Option<usize>,
    /// 预留：v0.4 reflection / compilation 用小模型时覆盖
    pub model: Option<String>,
}

impl SubagentProfile {
    pub fn read_only_tool_names() -> &'static [&'static str] {
        &["read_file", "search_code", "glob", "list_dir", "web_fetch"]
    }
}

/// 5 个内置 profile（Kun `BUILTIN_SUBAGENT_PROFILES` 对齐）
pub fn builtin_profiles() -> Vec<SubagentProfile> {
    vec![
        SubagentProfile {
            name: "general".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::Inherit,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「通用代理」(General)。",
                "你能研究复杂问题并执行多步骤任务，拥有与主代理一致的完整工具访问权限。",
                "适合被派去并行承担一个独立的工作单元。",
                "聚焦交给你的具体任务，完成后简洁汇报结果与关键改动。",
            )
            .into(),
            description: "通用代理：研究复杂问题、执行多步骤任务，可读写文件、运行命令。".into(),
            default_tools: vec![],
            max_iterations: None,
            model: None,
        },
        SubagentProfile {
            name: "explore".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「探索代理」(Explore)，一个快速的只读代码库代理。",
                "你只读取/搜索/列目录/抓网页，绝不修改任何文件。",
                "当需要按模式快速查找文件、搜索代码关键字、或回答关于代码库的问题时使用你。",
                "高效定位相关位置，返回结论（文件:行 + 简要说明），不做与任务无关的展开。",
            )
            .into(),
            description: "只读探索代理：快速查找文件、搜索关键字、回答关于代码库的问题。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            max_iterations: Some(15),
            model: None,
        },
        SubagentProfile {
            name: "code-writer".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::Inherit,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「代码编写代理」(Code Writer)。",
                "你专注于实现具体功能/修复 bug：读相关代码、设计方案、写入修改、运行测试验证。",
                "尊重现有代码风格、命名约定和依赖选型；不引入未经用户同意的新依赖。",
                "完成时汇报：改了哪些文件、关键设计决策、是否有未验证的风险。",
            )
            .into(),
            description: "代码编写代理：实现功能/修 bug，读写文件、运行测试。".into(),
            default_tools: vec![],
            max_iterations: None,
            model: None,
        },
        SubagentProfile {
            name: "planner".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「规划代理」(Planner)，一个纯推理规划角色。",
                "你不修改任何文件，只读代码、思考、设计。",
                "产出物：分步骤的实施计划，每步标明（输入/动作/输出/风险），",
                "识别依赖关系和可并行的工作单元。",
                "计划要具体到文件:函数级别，避免泛泛而谈。",
            )
            .into(),
            description: "规划代理：纯推理设计实施方案，不修改文件。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            max_iterations: Some(10),
            model: None,
        },
        SubagentProfile {
            name: "reviewer".into(),
            mode: SubagentMode::Subagent,
            tool_policy: ToolPolicy::ReadOnly,
            prompt_preamble: concat!(
                "你是 qingbird 内置的「审查代理」(Reviewer)。",
                "你只读代码并报告问题，不做任何修改。",
                "审查维度：正确性（含边界情况）、错误处理、可读性、与现有约定的一致性。",
                "每条问题给出：文件:行 + 问题描述 + 严重程度（critical/major/minor）+ 建议改法。",
                "按严重程度排序输出，不要泛泛而谈。",
            )
            .into(),
            description: "审查代理：只读代码审查问题，不修改。".into(),
            default_tools: SubagentProfile::read_only_tool_names()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            max_iterations: Some(12),
            model: None,
        },
    ]
}
