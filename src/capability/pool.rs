//! M10.5 多 Subagent 并发池 — Phase C Task C1
//!
//! 设计 §13.3 v1.1 形态：mpsc + N 个 worker task，并行派发 role + 能力组合。
//!
//! 注：tokio 1.x 的 `mpsc::Receiver` 不实现 `Clone`，
//! 多 worker 共享通过 `Arc<tokio::sync::Mutex<Receiver>>` 实现。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::capability::subagent::Subagent;
use crate::common::types::{Capability, Role};

/// Subagent 池请求
#[derive(Debug, Clone)]
pub enum PoolRequest {
    /// 派发一个角色 + 能力组合
    Dispatch {
        role: Role,
        capabilities: Vec<Capability>,
        reply: mpsc::Sender<Uuid>, // 返回 agent id
    },
    /// 关闭池
    Shutdown,
}

/// Subagent 池（设计 §13.3 v1.1 形态）
pub struct SubagentPool {
    tx: mpsc::Sender<PoolRequest>,
    /// 池中活跃 agent
    active: Arc<std::sync::Mutex<HashMap<Uuid, Subagent>>>,
}

impl SubagentPool {
    /// 启动池（spawn N 个 worker task）
    pub fn start(worker_count: usize) -> Self {
        let (tx, rx) = mpsc::channel::<PoolRequest>(64);
        let active: Arc<std::sync::Mutex<HashMap<Uuid, Subagent>>> =
            Arc::new(std::sync::Mutex::new(HashMap::new()));
        let rx = Arc::new(Mutex::new(rx));

        for worker_id in 0..worker_count {
            let rx = rx.clone();
            let active = active.clone();
            tokio::spawn(async move {
                tracing::info!("SubagentPool worker {} started", worker_id);
                loop {
                    let req = {
                        let mut guard = rx.lock().await;
                        guard.recv().await
                    };
                    let Some(req) = req else { break };
                    match req {
                        PoolRequest::Dispatch {
                            role,
                            capabilities,
                            reply,
                        } => {
                            let agent = Subagent::new(
                                format!("worker-{}-agent", worker_id),
                                role,
                                capabilities,
                            );
                            let id = agent.id;
                            active.lock().unwrap().insert(id, agent);
                            let _ = reply.send(id).await;
                        }
                        PoolRequest::Shutdown => break,
                    }
                }
                tracing::info!("SubagentPool worker {} stopped", worker_id);
            });
        }

        Self { tx, active }
    }

    /// 派发一个新 agent
    pub async fn dispatch(
        &self,
        role: Role,
        capabilities: Vec<Capability>,
    ) -> Result<Uuid, crate::common::error::EflowError> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        self.tx
            .send(PoolRequest::Dispatch {
                role,
                capabilities,
                reply: reply_tx,
            })
            .await
            .map_err(|_| crate::common::error::EflowError::Internal("pool closed".into()))?;
        reply_rx
            .recv()
            .await
            .ok_or_else(|| crate::common::error::EflowError::Internal("no reply".into()))
    }

    /// 列出活跃 agent 数量
    pub fn active_count(&self) -> usize {
        self.active.lock().unwrap().len()
    }

    /// 取出 agent 并返回句柄（drop 时自动归还）
    pub fn take_handle(&self, id: Uuid) -> Option<SubagentHandle> {
        let map = self.active.lock().unwrap();
        if map.contains_key(&id) {
            Some(SubagentHandle {
                id,
                pool_active: self.active.clone(),
            })
        } else {
            None
        }
    }

    /// 优雅关闭
    pub async fn shutdown(&self) {
        let _ = self.tx.send(PoolRequest::Shutdown).await;
    }

    /// Idle cleanup — v1.1 简化：handle drop 时已自动归还
    ///
    /// v1.1 实现：返回 0（无超时机制，handle drop 即清理路径已覆盖）
    /// v1.2 计划：加 timeout-based 清理，长时间空闲的 agent 主动回收
    /// （设计 §13.3 idle cleanup 子节）
    pub fn cleanup_idle(&self) -> usize {
        0
    }
}

/// 句柄：drop 时自动归还 agent 到池
pub struct SubagentHandle {
    id: Uuid,
    pool_active: Arc<std::sync::Mutex<HashMap<Uuid, Subagent>>>,
}

impl SubagentHandle {
    #[must_use]
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// v1.2 E1: 暴露 pool active map 的 guard，让 Orchestrator 拿到 agent 角色 /
    /// 能力做调度决策。返回 guard (caller 借用到 &self)，caller 用法:
    /// ```ignore
    /// let map = h.subagent().expect("alive");
    /// let sa = map.get(&h.id()).expect("present");
    /// ```
    /// std::sync::Mutex 的 guard 释放后 &Subagent 借用即失效 (E0515),
    /// 所以返回 guard 让 caller 控制借用期。caller 不能跨 await 持有
    /// (std mutex 不支持) —— Orchestrator 同步取角色 / 能力路径适用。
    /// Drop 语义保护: handle 活着 → active map 还在; handle drop 时
    /// map.remove(self.id) 才会发生。
    #[must_use]
    pub fn subagent(&self) -> Option<std::sync::MutexGuard<'_, HashMap<Uuid, Subagent>>> {
        self.pool_active.lock().ok()
    }
}

impl Drop for SubagentHandle {
    fn drop(&mut self) {
        if let Ok(mut map) = self.pool_active.lock() {
            map.remove(&self.id);
        }
    }
}

/// Role → 默认 capability 映射
pub fn default_capabilities_for_role(role: Role) -> Vec<Capability> {
    match role {
        Role::FileAssistant => vec![Capability::ReadFile, Capability::WriteFile],
        Role::CodeAssistant => {
            vec![
                Capability::ReadFile,
                Capability::SearchCode,
                Capability::LlmReasoning,
            ]
        }
        Role::DataAnalyst => vec![Capability::ReadFile, Capability::LlmReasoning],
        Role::Generalist => vec![
            Capability::ReadFile,
            Capability::WriteFile,
            Capability::SearchCode,
            Capability::LlmReasoning,
        ],
    }
}

/// 便捷派发：用 role 默认 capability
impl SubagentPool {
    pub async fn dispatch_for_role(
        &self,
        role: Role,
    ) -> Result<Uuid, crate::common::error::EflowError> {
        let caps = default_capabilities_for_role(role.clone());
        self.dispatch(role, caps).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_dispatches_unique_agents() {
        let pool = SubagentPool::start(2);
        let id1 = pool
            .dispatch(Role::CodeAssistant, vec![Capability::ReadFile])
            .await
            .unwrap();
        let id2 = pool
            .dispatch(Role::FileAssistant, vec![Capability::WriteFile])
            .await
            .unwrap();
        assert_ne!(id1, id2);
        assert_eq!(pool.active_count(), 2);
        pool.shutdown().await;
    }

    #[tokio::test]
    async fn handle_drop_returns_agent_to_pool_inactive() {
        let pool = SubagentPool::start(2);
        let id = pool.dispatch(Role::Generalist, vec![]).await.unwrap();
        assert_eq!(pool.active_count(), 1);
        {
            let _h = pool.take_handle(id).unwrap();
            assert_eq!(pool.active_count(), 1);
        }
        // drop 后
        assert_eq!(pool.active_count(), 0);
        pool.shutdown().await;
    }

    #[tokio::test]
    async fn cleanup_idle_is_noop_in_v1_1() {
        // v1.1 简化：handle drop 已即时归还，cleanup_idle 是占位
        // 验证：fresh pool 调用不 panic，返回 0
        let pool = SubagentPool::start(2);
        assert_eq!(pool.cleanup_idle(), 0);
    }

    // v1.2 E1: SubagentHandle.subagent() 暴露 pool active map 的 guard,
    // 让 Orchestrator 拿到 agent 角色 / 能力做调度决策
    #[tokio::test]
    async fn handle_exposes_subagent_reference() {
        use crate::common::types::Capability;
        let pool = SubagentPool::start(2);
        let id = pool
            .dispatch(Role::CodeAssistant, vec![Capability::ReadFile])
            .await
            .unwrap();
        let h = pool.take_handle(id).unwrap();
        // subagent() 返回 guard; 通过 guard 取 entry
        {
            let map = h
                .subagent()
                .expect("handle should expose pool active map while alive");
            let sa = map.get(&h.id()).expect("entry present while handle alive");
            // 用 matches! 而非 assert_eq! —— Role 当前未 derive PartialEq (out of E1 scope)
            assert!(matches!(sa.role, Role::CodeAssistant));
            assert!(sa.capabilities.contains(&Capability::ReadFile));
            // map (MutexGuard) 在这个 scope 结束处 drop —— 避免跨 await
        }
        drop(h);
        // drop 后 active map 清空
        assert_eq!(pool.active_count(), 0);
        pool.shutdown().await;
    }
}

#[cfg(test)]
mod role_routing_tests {
    use super::*;

    #[test]
    fn code_assistant_includes_search_and_llm() {
        let caps = default_capabilities_for_role(Role::CodeAssistant);
        assert!(caps.contains(&Capability::SearchCode));
        assert!(caps.contains(&Capability::LlmReasoning));
        assert!(!caps.contains(&Capability::WriteFile));
    }

    #[test]
    fn file_assistant_has_read_and_write_only() {
        let caps = default_capabilities_for_role(Role::FileAssistant);
        assert!(caps.contains(&Capability::ReadFile));
        assert!(caps.contains(&Capability::WriteFile));
        assert!(!caps.contains(&Capability::ExecuteCommand));
    }
}
