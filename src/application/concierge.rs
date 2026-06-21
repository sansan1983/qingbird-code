use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::application::orchestrator::Orchestrator;
use crate::common::types::Intent;
use crate::common::types::{RiskLevel, TaskSpec};
use crate::infrastructure::event::{Event, EventChannel};
use crate::infrastructure::llm::LlmRouter;
use crate::infrastructure::memory::CompositeMemory;
use rust_i18n::t;

/// Concierge — 零阻塞对话入口
pub struct Concierge {
    events: EventChannel,
    // v1.2 D4: 删 dead_code 注解——handle_input 的 TaskDispatch 分支调 memory.recall_smart
    memory: Arc<Mutex<CompositeMemory>>,
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
    pub command_registry: crate::interaction::slash::CommandRegistry,
}

impl Concierge {
    pub fn new(
        events: EventChannel,
        memory: Arc<Mutex<CompositeMemory>>,
        orchestrator: Arc<Mutex<Orchestrator>>,
        llm_router: Arc<Mutex<LlmRouter>>, // v1.3.1 增量
        default_profile: String,
    ) -> Self {
        Self {
            events,
            memory,
            orchestrator,
            llm_router, // v1.3.1 增量
            active_profile: Arc::new(Mutex::new(default_profile)),
            command_registry: crate::interaction::slash::CommandRegistry::new(), // v1.3.1 增量
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

        let llm = Arc::new(Mutex::new(LlmRouter::placeholder()));
        let tools = Arc::new(ToolRegistry::new());
        let events = EventChannel::new();
        let memory = Arc::new(Mutex::new(CompositeMemory::in_memory(10).unwrap()));
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new(
            llm.clone(),
            tools,
            events.clone(),
        )));

        Self::new(events, memory, orchestrator, llm, "default".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::Intent;

    #[test]
    fn classify_default_goes_to_task_dispatch() {
        let concierge = Concierge::placeholder();
        // 不含任何关键词的 input → 默认 TaskDispatch
        let intent = concierge.classify_intent("review the auth module");
        assert!(matches!(intent, Intent::TaskDispatch { .. }));
    }

    #[test]
    fn classify_chinese_long_input_goes_to_task_dispatch() {
        let concierge = Concierge::placeholder();
        let intent =
            concierge.classify_intent("请帮我处理一个长任务描述，超过 30 字符以触发标准派发路径");
        assert!(matches!(intent, Intent::TaskDispatch { .. }));
    }

    #[test]
    fn classify_profile_switch_intent_zh() {
        let concierge = Concierge::placeholder();
        let intent = concierge.classify_intent("切换到 backend profile");
        assert!(matches!(intent, Intent::ProfileSwitch { .. }));
    }

    #[test]
    fn classify_task_cancel_intent_zh() {
        let concierge = Concierge::placeholder();
        let intent = concierge.classify_intent("取消 任务 abc");
        assert!(matches!(intent, Intent::TaskCancel { .. }));
    }

    #[test]
    fn classify_task_interrupt_intent_zh() {
        let concierge = Concierge::placeholder();
        let intent = concierge.classify_intent("中断当前任务");
        assert!(matches!(intent, Intent::TaskInterrupt { .. }));
    }

    #[test]
    fn classify_skill_query_intent_en() {
        let concierge = Concierge::placeholder();
        let intent = concierge.classify_intent("query skill xyz");
        assert!(matches!(intent, Intent::SkillQuery { .. }));
    }

    #[test]
    fn handle_input_default_returns_dispatched_ack() {
        // TaskDispatch 分支内部 tokio::spawn 异步派发 — 需要真 runtime
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt build");
        let mut concierge = Concierge::placeholder();
        let ack = rt.block_on(
            concierge.handle_input("some long input that goes to task dispatch path".into()),
        );
        // TaskDispatch 分支返回 t!("concierge_task_dispatched", ...) 文本
        assert!(!ack.is_empty());
    }

    #[test]
    fn active_profile_initially_default() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt build");
        let concierge = Concierge::placeholder();
        let p = rt.block_on(concierge.active_profile());
        assert_eq!(p, "default");
    }

    #[test]
    fn subscribe_events_returns_receiver() {
        let concierge = Concierge::placeholder();
        let mut rx = concierge.subscribe_events();
        // 空 broadcast channel：try_recv 立即返 Empty（sender 还活着）
        let r = rx.try_recv();
        assert!(matches!(
            r,
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    }
}
