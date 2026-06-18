//! end handler（占位）—— T6 真实现：cancel task
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(_concierge: &mut Concierge, _task_id: Uuid) -> Result<()> {
    Ok(())
}
