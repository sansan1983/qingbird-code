//! v1.3.2 stdin 协议（占位）—— 真实现见 T5
//!
//! 当前只暴露一个 `read_loop` 让 `start.rs` import 链通；M4 T5/T6 将
//! 替换为完整的 5-action 解析 + 5 handlers。
//!
//! 关键设计决策（spec B2）：
//! - 5 个 action JSON：send / end / level / lang / help
//! - 解析失败不退出（stdin 网络抖动时 GUI 偶尔发坏 JSON 不该让 eflow 死）
//! - 用 `#[serde(tag = "action")]` 做 enum 标签

use crate::application::concierge::Concierge;

/// 占位：T5 之前让 build 通；T5 替换为「读 stdin 一行行 → StdinCommand 解析 → handler dispatch」
pub async fn read_loop(_concierge: &mut Concierge) -> i32 {
    0
}
