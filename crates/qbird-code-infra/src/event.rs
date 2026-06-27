use qbird_code_models::RiskLevel;
use tokio::sync::broadcast;
use uuid::Uuid;

/// 事件通道缓冲区大小（fix v1.0.3 M3 抽离）
const EVENT_BUFFER_SIZE: usize = 256;

/// 系统事件
#[derive(Debug, Clone)]
pub enum Event {
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
mod tests {}
