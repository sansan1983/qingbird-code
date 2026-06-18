use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::application::orchestrator::Orchestrator;
use crate::common::types::Intent;
use crate::common::types::{RiskLevel, TaskSpec};
use crate::infrastructure::event::{Event, EventChannel};
use crate::infrastructure::llm::LlmRouter;
use crate::infrastructure::memory::CompositeMemory;
use crate::infrastructure::profile::ProfileRegistry;
use rust_i18n::t;

/// Concierge — 零阻塞对话入口
pub struct Concierge {
    events: EventChannel,
    // v1.2 D4: 删 dead_code 注解——handle_input 的 TaskDispatch 分支调 memory.recall_smart
    memory: Arc<Mutex<CompositeMemory>>,
    // v1.2 D3: 仍 dead（D3/D4 都不直接读 profiles，只用 active_profile 字符串）
    // ——Phase D 收尾时若仍无读取点，应考虑移除该字段；v1.2 阶段先保留
    #[allow(dead_code)]
    profiles: Arc<tokio::sync::RwLock<ProfileRegistry>>,
    orchestrator: Arc<Mutex<Orchestrator>>,
    // v1.3.1 增量：LlmRouter 共享给 SlashCommand——通过 Arc<Mutex<>> 让
    // CommandContext 借用期间不冲突（v1.3.0 Concierge **不**持 router 字段，
    // 由 setter 注入）
    llm_router: Arc<Mutex<LlmRouter>>,
    // v1.2 D3: 用 Mutex 包裹让 ProfileSwitch 意图能真改；通过 active_profile()
    // getter 和 handle_input 的 ProfileSwitch 分支都用到，不再 dead
    active_profile: Arc<Mutex<String>>,
    // v1.3.1 增量：斜杠命令注册表——/model /profile /lang /level /help /quit
    // 默认空（v1.2 调用点不传），main.rs 启动时填充
    #[allow(dead_code)]
    pub command_registry: crate::interaction::slash::CommandRegistry,
    // v1.3.3 增量：工作流档位注册表——Simple/Standard/Advanced 3 个 impl
    // 默认空（v1.2 调用点不传），main.rs 启动时通过 set_workflow_registry 注入
    #[allow(dead_code)]
    pub workflow_registry: crate::workflow::WorkflowRegistry,
}

impl Concierge {
    pub fn new(
        events: EventChannel,
        memory: Arc<Mutex<CompositeMemory>>,
        profiles: Arc<tokio::sync::RwLock<ProfileRegistry>>,
        orchestrator: Arc<Mutex<Orchestrator>>,
        llm_router: Arc<Mutex<LlmRouter>>, // v1.3.1 增量
        default_profile: String,
    ) -> Self {
        Self {
            events,
            memory,
            profiles,
            orchestrator,
            llm_router, // v1.3.1 增量
            active_profile: Arc::new(Mutex::new(default_profile)),
            command_registry: crate::interaction::slash::CommandRegistry::new(), // v1.3.1 增量
            workflow_registry: crate::workflow::WorkflowRegistry::default(),     // v1.3.3 增量
        }
    }

    /// v1.2 D3: 暴露 active_profile 给测试和 UI 读取
    pub async fn active_profile(&self) -> String {
        self.active_profile.lock().await.clone()
    }

    /// v1.3.2: 暴露 events 给 headless CLI（start.rs）订阅
    /// —— Concierge 持有 EventChannel 所有权，外部只读订阅
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// v1.3.3: 暴露 llm_router Arc clone 给 WorkflowExecutor 借用
    /// —— SimpleWorkflow 1 次 LLM 调用通过这个拿锁（与 self 借用独立）
    pub fn llm_router_handle(&self) -> Arc<tokio::sync::Mutex<LlmRouter>> {
        Arc::clone(&self.llm_router)
    }

    /// v1.3.1: 公开 setter 让 `/profile` 斜杠命令能真切换
    pub async fn set_active_profile(&self, name: String) {
        let mut p = self.active_profile.lock().await;
        *p = name;
    }

    /// 处理用户输入 — 永不阻塞：派发任务用 `tokio::spawn` 异步执行
    ///
    /// v1.3.1 增量：`/` 前缀的输入走 `command_registry` 斜杠命令分发；
    /// 非斜杠输入走原 v1.2 意图分类路径。
    pub async fn handle_input(&mut self, input: String) -> String {
        // v1.3.1 增量：斜杠命令优先
        if let Some(cmd_str) = input.strip_prefix('/') {
            return self.dispatch_slash(cmd_str).await;
        }

        let intent = self.classify_intent(&input);

        match intent {
            Intent::Chat { content } => {
                t!("concierge_chat_received", content = content).to_string()
            }
            Intent::TaskDispatch { spec } => {
                // v1.2 D4: 派发前 recall 相关历史记忆（设计 §7.2）
                // 关键词取 task description 前 32 字符
                let keyword: String = spec.description.chars().take(32).collect();
                let mem_snapshot: Vec<String> = {
                    let mem = self.memory.lock().await;
                    mem.recall_smart(&keyword, 3)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|e| e.content)
                        .collect()
                };

                let task_id = spec.id;
                // 异步派发任务，不等待结果
                let orch = self.orchestrator.clone();
                let events = self.events.clone();
                tokio::spawn(async move {
                    let mut orch = orch.lock().await;
                    match orch.execute(spec).await {
                        Ok(summary) => {
                            events.publish(Event::TaskCompleted { task_id, summary });
                        }
                        Err(e) => {
                            events.publish(Event::TaskFailed {
                                task_id,
                                error: e.to_string(),
                            });
                        }
                    }
                });
                let id_prefix: String = task_id.to_string().chars().take(8).collect();
                let mem_count = mem_snapshot.len();
                if mem_count > 0 {
                    t!(
                        "concierge_task_dispatched_with_memory",
                        id = id_prefix,
                        count = mem_count
                    )
                    .to_string()
                } else {
                    t!("concierge_task_dispatched", id = id_prefix).to_string()
                }
            }
            Intent::TaskInterrupt { task_id } => {
                t!("concierge_task_interrupt", id = task_id).to_string()
            }
            Intent::TaskCancel { task_id } => t!("concierge_task_cancel", id = task_id).to_string(),
            Intent::SkillQuery { keyword } => self.list_skills(&keyword),
            Intent::ProfileSwitch { industry } => {
                // v1.2 D3: 真改 active_profile，不再只发提示
                let mut p = self.active_profile.lock().await;
                *p = industry.clone();
                t!("concierge_profile_switch", industry = industry).to_string()
            }
        }
    }

    /// v1.3.1 增量：分发斜杠命令到 CommandRegistry
    ///
    /// 关键：`&mut self` 因为 `CommandContext` 持有 `&mut Concierge`。
    /// router 通过 `CommandContext.router: Arc<Mutex<LlmRouter>>` 共享——execute
    /// 内部需要时再 lock（plan deviation：原计划 `&mut LlmRouter` 借用冲突，
    /// v1.3.1 改成 `Arc<Mutex<>>`）。
    /// v1.3.2 T6: 改 pub 让 cli::handlers::level 能复用 SlashCommand 链路
    /// —— v1.3.1 是 async fn（private），surgical 改 visibility 而非复制逻辑
    pub async fn dispatch_slash(&mut self, cmd_str: &str) -> String {
        use crate::interaction::slash::{CommandContext, SlashOutput};

        match self.command_registry.dispatch(cmd_str) {
            Some((name, args)) => {
                let cmd_arc = match self.command_registry.get(name) {
                    Some(c) => c.clone(), // clone Arc 释放 &self 借用
                    None => {
                        // v1.3.1 T11: 用 err_unknown_slash_cmd i18n key（fallback en-US）
                        return t!("err_unknown_slash_cmd", name = name).to_string();
                    }
                };

                // router Arc<Mutex> clone 释放 self 借用
                let router_arc = Arc::clone(&self.llm_router);
                let mut ctx = CommandContext::new(self, router_arc);

                match cmd_arc.execute(args, &mut ctx).await {
                    Ok(output) => match output {
                        SlashOutput::Text(s) => s,
                        SlashOutput::NoOp => String::new(),
                        // v1.3.1 阶段: ReloadRouter/Shutdown/OpenSubView 3 个输出未实装，
                        // 走 err_subview_render_failed / err_cmd_failed i18n key 兜底
                        // —— v1.3.3 spec B 实施时由真调用方替换
                        SlashOutput::ReloadRouter => {
                            t!("err_cmd_failed", msg = "ReloadRouter").to_string()
                        }
                        SlashOutput::Shutdown => t!("err_cmd_failed", msg = "Shutdown").to_string(),
                        SlashOutput::OpenSubView(_) => {
                            t!("err_subview_render_failed", msg = "OpenSubView").to_string()
                        }
                    },
                    Err(e) => t!("err_cmd_failed", msg = e.to_string()).to_string(),
                }
            }
            None => t!("err_unknown_slash_cmd", name = cmd_str).to_string(),
        }
    }

    /// 规则驱动的意图分类（v1.0：不调 LLM）
    #[must_use]
    pub fn classify_intent(&self, input: &str) -> Intent {
        let input_lower = input.to_lowercase();

        if input_lower.contains("切换") && input_lower.contains("profile") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            let name = parts.last().unwrap_or(&"developer");
            return Intent::ProfileSwitch {
                industry: name.to_string(),
            };
        }
        if input_lower.contains("取消") && input_lower.contains("任务") {
            // v1.0 简化：未跟踪 task id 列表，用 nil 标记"无目标"
            return Intent::TaskCancel {
                task_id: Uuid::nil(),
            };
        }
        if input_lower.contains("中断") {
            return Intent::TaskInterrupt {
                task_id: Uuid::nil(),
            };
        }
        if input_lower.contains("skill") || input_lower.contains("技能") {
            return Intent::SkillQuery {
                keyword: input.to_string(),
            };
        }

        // 默认：任务派发
        let spec = TaskSpec::new(input.to_string(), RiskLevel::L0);
        Intent::TaskDispatch { spec }
    }

    fn list_skills(&self, _keyword: &str) -> String {
        // v1.0 简化：不持锁扫描 profile.skill 列表
        t!("concierge_skill_query_placeholder").to_string()
    }

    /// v1.3.3 增量：注入工作流档位注册表（main.rs 启动时调用）
    pub fn set_workflow_registry(&mut self, registry: crate::workflow::WorkflowRegistry) {
        self.workflow_registry = registry;
    }

    /// v1.3.3 增量：可变借用 workflow_registry（/level 命令用）
    pub fn workflow_registry_mut(&mut self) -> &mut crate::workflow::WorkflowRegistry {
        &mut self.workflow_registry
    }

    /// v1.3.3 增量：规则驱动的档位判定（5 条规则，按优先级匹配）
    ///
    /// **零 LLM 成本**——纯字符串匹配
    /// **override 不在此处检查**——调用方负责：
    ///   `override.unwrap_or(self.determine_workflow_level(task))`
    pub fn determine_workflow_level(
        &self,
        task: &crate::common::types::TaskSpec,
    ) -> crate::workflow::WorkflowLevel {
        let desc = &task.description;
        let len = desc.chars().count();

        // 规则 1: 涉及多文件（≥ 3 个扩展名）→ Advanced
        if count_file_extensions(desc) >= 3 {
            return crate::workflow::WorkflowLevel::Advanced;
        }

        // 规则 2: 含关键词（中英）→ Advanced
        if contains_workflow_keyword(desc) {
            return crate::workflow::WorkflowLevel::Advanced;
        }

        // 规则 3: 短任务（< 30 字符）→ Simple
        if len < 30 {
            return crate::workflow::WorkflowLevel::Simple;
        }

        // 规则 4: 中等任务（30-100 字符）→ Standard
        if len < 100 {
            return crate::workflow::WorkflowLevel::Standard;
        }

        // 规则 5: 长任务（≥ 100 字符）→ Advanced
        crate::workflow::WorkflowLevel::Advanced
    }

    /// v1.3.3 增量：派发任务到 workflow_registry（override 优先 + 调档位 execute）
    ///
    /// **借用模式说明**：ctx.concierge 是 `&mut self` 独占借用，期间不能访问
    /// self.workflow_registry。解法：先 `override_level()` 算出 level（&self 借用
    /// 立即释放），再用 `std::mem::take` 把 registry 移出到 owned 局部变量，
    /// 调完 execute 后 `drop(ctx)` + 放回 self。`take` 用 `Default::default()`
    /// 临时替换（空 registry），原内容在 `reg` 局部变量里保留，**不丢失**任何注册。
    pub async fn dispatch_task_with_level(
        &mut self,
        task: crate::common::types::TaskSpec,
    ) -> crate::common::error::Result<crate::workflow::AggregatedResult> {
        // 1. 算 level（&self.workflow_registry 借用，结束于 ;）
        let auto_level = self.determine_workflow_level(&task);
        let level = self
            .workflow_registry
            .override_level()
            .unwrap_or(auto_level);

        // 2. take registry 出来 owned —— &mut self.workflow_registry 借用结束于 ;
        let reg = std::mem::take(&mut self.workflow_registry);

        // 3. 准备 Arc 字段（&self split borrow，OK）
        let orch_arc = Arc::clone(&self.orchestrator);
        let mem_arc = Arc::clone(&self.memory);

        // 4. 构造 ctx —— &mut self 借用
        let mut ctx = crate::workflow::WorkflowContext {
            task: &task,
            concierge: self,
            orchestrator: orch_arc,
            memory: mem_arc,
        };

        // 5. 调 execute（reg 是 owned 局部变量，与 ctx 独立）
        let result = reg.execute(level, &mut ctx).await;

        // 6. 释放 ctx 借用后，把 reg 放回 self
        drop(ctx);
        self.workflow_registry = reg;

        result
    }
}

/// 统计任务描述里的文件扩展名数量（≥ 3 → Advanced）
fn count_file_extensions(s: &str) -> usize {
    const EXTENSIONS: &[&str] = &[
        "rs", "py", "js", "ts", "go", "java", "cpp", "c", "h", "toml", "yaml", "yml", "json", "md",
    ];
    EXTENSIONS
        .iter()
        .map(|ext| s.matches(&format!(".{ext}")).count())
        .sum()
}

/// 检测任务描述是否含"重构/系统/全部/refactor..."关键词（→ Advanced）
///
/// v1.3.3 #13k: case-insensitive（spec C §3.3 设计意图）—— 英文 keyword
/// "refactor" 应同时匹配 "Refactor" / "REFACTOR"。
fn contains_workflow_keyword(s: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "重构",
        "系统",
        "全部",
        "梳理",
        "优化",
        "refactor",
        "refactoring",
        "system",
        "all",
        "optimize",
    ];
    let s_lower = s.to_lowercase();
    KEYWORDS.iter().any(|kw| s_lower.contains(kw))
}

/// 测试用 placeholder Concierge（v1.3.3 deviation #13d: v1.3.0 没加，
/// v1.3.3 spec C 实施时补——参考 v1.3.1 LlmRouter::placeholder 模式）
///
/// **非测试代码不应调用**——用 `Concierge::new`。
#[doc(hidden)]
impl Concierge {
    #[must_use]
    pub fn placeholder() -> Self {
        use crate::application::orchestrator::Orchestrator;
        use crate::capability::tools::ToolRegistry;
        use crate::infrastructure::event::EventChannel;
        use crate::infrastructure::llm::LlmRouter;
        use crate::infrastructure::memory::CompositeMemory;
        use crate::infrastructure::profile::ProfileRegistry;

        let llm = Arc::new(Mutex::new(LlmRouter::placeholder()));
        let tools = Arc::new(ToolRegistry::new());
        let events = EventChannel::new();
        let memory = Arc::new(Mutex::new(CompositeMemory::in_memory(10).unwrap()));
        let profiles = Arc::new(tokio::sync::RwLock::new(ProfileRegistry::default()));
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new(
            llm.clone(),
            tools,
            events.clone(),
        )));

        Self::new(
            events,
            memory,
            profiles,
            orchestrator,
            llm,
            "default".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::{RiskLevel, TaskSpec};
    use crate::workflow::WorkflowLevel;

    fn make_task(desc: &str) -> TaskSpec {
        TaskSpec::new(desc.to_string(), RiskLevel::L0)
    }

    #[test]
    fn short_task_under_30_chars_is_simple() {
        let concierge = Concierge::placeholder();
        let task = make_task("hi");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Simple
        );
    }

    #[test]
    fn medium_task_30_to_100_chars_is_standard() {
        let concierge = Concierge::placeholder();
        let task = make_task("帮我看看 main.rs 文件在做什么事情，简单的代码审查一下");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Standard
        );
    }

    #[test]
    fn long_task_over_100_chars_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task(
            "请帮我对整个项目的代码进行一次系统性的梳理和优化，包括所有的源文件、\
             测试文件、配置文件等等，需要全面地分析代码质量、性能瓶颈、安全漏洞，\
             并给出详细的改进建议和实施计划",
        );
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }

    #[test]
    fn task_with_refactor_keyword_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task("重构 auth 模块");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }

    #[test]
    fn task_with_english_refactor_keyword_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task("Refactor the entire codebase");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }

    #[test]
    fn task_with_3_rs_files_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task("改 main.rs lib.rs config.rs");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }

    #[test]
    fn task_with_3_py_files_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task("fix app.py models.py tests.py");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }

    #[test]
    fn task_with_3_different_extensions_is_advanced() {
        let concierge = Concierge::placeholder();
        let task = make_task("review Cargo.toml README.md src/main.rs");
        assert_eq!(
            concierge.determine_workflow_level(&task),
            WorkflowLevel::Advanced
        );
    }
}
