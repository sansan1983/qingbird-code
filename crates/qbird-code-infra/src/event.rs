use qbird_code_models::RiskLevel;
use tokio::sync::broadcast;
use uuid::Uuid;

/// 事件通道缓冲区大小（fix v1.0.3 M3 抽离）
const EVENT_BUFFER_SIZE: usize = 256;

/// 系统事件
#[derive(Debug, Clone)]
pub enum Event {
    /// v1.3.2 增量：start 启动后第一行输出，GUI 用此判断"qingbird 启动完成"
    /// —— v1.3.2 当前未在 event channel 流通（start.rs 手写 NDJSON）；
    /// 保留 variant 以便将来按事件流分发时复用。
    SystemReady {
        task_id: Uuid,
        started_at: std::time::SystemTime,
    },
    TaskStarted {
        task_id: Uuid,
        description: String,
    },
    TaskCompleted {
        task_id: Uuid,
        summary: String,
    },
    TaskFailed {
        task_id: Uuid,
        error: String,
    },
    RiskEscalated {
        task_id: Uuid,
        from: RiskLevel,
        to: RiskLevel,
    },
    UserInputRequired {
        prompt: String,
    },
    SystemShutdown,
}

/// 事件通道 — 基于 `tokio::broadcast`
pub struct EventChannel {
    tx: broadcast::Sender<Event>,
}

impl EventChannel {
    /// 创建新通道
    #[must_use]
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(EVENT_BUFFER_SIZE);
        Self { tx }
    }

    /// 发布事件（不等待，忽略无订阅者错误）
    pub fn publish(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    /// 订阅事件流
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl Default for EventChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventChannel {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    // v1.3.2 T8: SystemReady variant 可构造 + 通过 EventChannel 流通
    // —— 验证：构造（Debug 派生） + 走 broadcast channel 收到一致
    #[test]
    fn system_ready_event_constructs_and_preserves_fields() {
        let event = Event::SystemReady {
            task_id: Uuid::nil(),
            started_at: UNIX_EPOCH + Duration::from_secs(1700000000),
        };
        // Debug derive 应正常格式化
        let debug = format!("{event:?}");
        assert!(debug.contains("SystemReady"));
    }

    #[test]
    fn system_ready_event_publishes_through_channel() {
        let channel = EventChannel::new();
        let mut rx = channel.subscribe();

        let event = Event::SystemReady {
            task_id: Uuid::nil(),
            started_at: UNIX_EPOCH,
        };
        channel.publish(event.clone());

        // try_recv 是 sync（非 async 上下文）；publish 同步 push 到 channel
        match rx.try_recv() {
            Ok(Event::SystemReady {
                task_id,
                started_at,
            }) => {
                assert_eq!(task_id, Uuid::nil());
                assert_eq!(started_at, UNIX_EPOCH);
            }
            Ok(other) => panic!("expected SystemReady, got {other:?}"),
            Err(e) => panic!("try_recv failed: {e}"),
        }
    }
}
