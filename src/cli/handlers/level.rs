//! level handler（占位）—— T6 真实现：设置 risk level
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(_concierge: &mut Concierge, _task_id: Uuid, _level: &str) -> Result<()> {
    Ok(())
}
