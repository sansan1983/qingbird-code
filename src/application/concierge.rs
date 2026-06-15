use std::sync::Arc;

use uuid::Uuid;

use crate::application::orchestrator::Orchestrator;
use crate::common::types::Intent;
use crate::common::types::*;
use crate::infrastructure::event::{Event, EventChannel};
use crate::infrastructure::memory::CompositeMemory;
use crate::infrastructure::profile::ProfileRegistry;
use rust_i18n::t;

/// Concierge — 零阻塞对话入口
pub struct Concierge {
    events: EventChannel,
    #[allow(dead_code)]
    memory: Arc<tokio::sync::Mutex<CompositeMemory>>,
    #[allow(dead_code)]
    profiles: Arc<tokio::sync::RwLock<ProfileRegistry>>,
    orchestrator: Arc<tokio::sync::Mutex<Orchestrator>>,
    #[allow(dead_code)]
    active_profile: String,
}

impl Concierge {
    pub fn new(
        events: EventChannel,
        memory: Arc<tokio::sync::Mutex<CompositeMemory>>,
        profiles: Arc<tokio::sync::RwLock<ProfileRegistry>>,
        orchestrator: Arc<tokio::sync::Mutex<Orchestrator>>,
        default_profile: String,
    ) -> Self {
        Self {
            events,
            memory,
            profiles,
            orchestrator,
            active_profile: default_profile,
        }
    }

    /// 处理用户输入 — 永不阻塞：派发任务用 tokio::spawn 异步执行
    pub async fn handle_input(&self, input: String) -> String {
        let intent = self.classify_intent(&input);

        match intent {
            Intent::Chat { content } => {
                t!("concierge_chat_received", content = content).to_string()
            }
            Intent::TaskDispatch { spec } => {
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
                t!("concierge_task_dispatched", id = id_prefix).to_string()
            }
            Intent::TaskInterrupt { task_id } => {
                t!("concierge_task_interrupt", id = task_id).to_string()
            }
            Intent::TaskCancel { task_id } => t!("concierge_task_cancel", id = task_id).to_string(),
            Intent::SkillQuery { keyword } => self.list_skills(&keyword),
            Intent::ProfileSwitch { industry } => {
                t!("concierge_profile_switch", industry = industry).to_string()
            }
        }
    }

    /// 规则驱动的意图分类（v1.0：不调 LLM）
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
