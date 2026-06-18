//! lang handler（占位）—— T6 真实现：切语言
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use uuid::Uuid;

pub async fn dispatch(
    _concierge: &mut Concierge,
    _task_id: Option<Uuid>,
    _locale: &str,
) -> Result<()> {
    Ok(())
}
