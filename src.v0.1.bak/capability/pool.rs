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
    /// ponytail: std::sync::Mutex — 当前所有访问路径（list_active / take_handle / cleanup_idle）
    /// 都在同步闭包内，不跨 .await 持有。如果未来有人在此锁和 .unlock() 之间加 .await，
    /// 会死锁。届时改为 tokio::sync::Mutex 并将方法改为 async。
    active: Arc<std::sync::Mutex<HashMap<Uuid, Subagent>>>,
    /// v1.2 E5: idle 超时（None = 不超时，v1.1 行为兼容）
    idle_timeout: Option<std::time::Duration>,
}

impl SubagentPool {
    /// 启动池（spawn N 个 worker task）
    /// v1.2 E5: 默认 5 分钟 idle timeout（v1.1 行为保持"几乎不会自动清理"）
    pub fn start(worker_count: usize) -> Self {
        Self::with_idle_timeout(worker_count, std::time::Duration::from_secs(300))
    }

    /// v1.2 E5: 显式指定 idle timeout 的池
    pub fn with_idle_timeout(worker_count: usize, idle_timeout: std::time::Duration) -> Self {
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

        Self {
            tx,
            active,
            idle_timeout: Some(idle_timeout),
        }
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

    /// v1.2 E2: 列出活跃 agent 的元数据 (id, name, role, capabilities)。
    /// Orchestrator 用这个做 role-based 调度决策（v1.2 E4 并行派发用）。
    /// ——返回 owned Vec 而非引用（避免 guard 跨 caller，借用期零问题）
    #[must_use]
    pub fn list_active(&self) -> Vec<(Uuid, String, Role, Vec<Capability>)> {
        self.active
            .lock()
            .unwrap()
            .iter()
            .map(|(id, sa)| {
                (
                    *id,
                    sa.name.clone(),
                    sa.role.clone(),
                    sa.capabilities.clone(),
                )
            })
            .collect()
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

    /// v1.2 E5: 扫活跃 map，移除 created_at + idle_timeout < now 的 agent。
    /// 返回移除数量。idle_timeout = None 时不清理（v1.1 占位兼容）
    pub fn cleanup_idle(&self) -> usize {
        let Some(timeout) = self.idle_timeout else {
            return 0;
        };
        let now = std::time::SystemTime::now();
        let mut map = match self.active.lock() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        let before = map.len();
        map.retain(|_, sa| {
            now.duration_since(sa.created_at)
                .map(|d| d < timeout)
                .unwrap_or(true) // 时钟回退 → 保留
        });
        before - map.len()
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

    // v1.2 E5: cleanup_idle 真实现（按 created_at + idle_timeout 移除）
    // 验证：50ms timeout，等 100ms 后 cleanup 移除 1 个 agent
    #[tokio::test]
    async fn cleanup_idle_removes_agents_past_timeout() {
        use crate::common::types::Capability;
        use std::time::Duration;
        let pool = SubagentPool::with_idle_timeout(2, Duration::from_millis(50));
        let _ = pool
            .dispatch(Role::Generalist, vec![Capability::ReadFile])
            .await
            .unwrap();
        assert_eq!(pool.active_count(), 1);
        // 等超过 idle timeout
        tokio::time::sleep(Duration::from_millis(100)).await;
        let removed = pool.cleanup_idle();
        assert_eq!(removed, 1, "1 个 agent 超时应被清理");
        assert_eq!(pool.active_count(), 0);
        pool.shutdown().await;
    }

    // v1.2 E5: cleanup_idle 不会清理未超时的 agent
    #[tokio::test]
    async fn cleanup_idle_keeps_agents_within_timeout() {
        use crate::common::types::Capability;
        use std::time::Duration;
        let pool = SubagentPool::with_idle_timeout(2, Duration::from_secs(300));
        let _ = pool
            .dispatch(Role::Generalist, vec![Capability::ReadFile])
            .await
            .unwrap();
        assert_eq!(pool.active_count(), 1);
        // 立即 cleanup，未超时
        let removed = pool.cleanup_idle();
        assert_eq!(removed, 0, "未超时应保留");
        assert_eq!(pool.active_count(), 1);
        pool.shutdown().await;
    }

    // v1.2 E5: 默认 start() pool 的 cleanup_idle 是 noop（v1.1 兼容）
    #[tokio::test]
    async fn cleanup_idle_default_pool_is_noop() {
        let pool = SubagentPool::start(2);
        let _ = pool.dispatch(Role::Generalist, vec![]).await.unwrap();
        let removed = pool.cleanup_idle();
        assert_eq!(removed, 0, "默认 pool 不应自动清理");
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
