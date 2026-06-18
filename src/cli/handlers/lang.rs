//! lang handler —— 切换语言
//!
//! 设计：直接调 `locale::init`（不绕 SlashCommand）—— GUI 触发切语言是真副作用，
//! 走 SlashCommand 还要经过 parse_args 校验（LangCmd 已校验 SUPPORTED_LOCALES），
//! 而 GUI 是 trusted caller（vs TUI 用户），不需要重复校验。
//!
//! `task_id` 字段：lang 不绑 task，spec 允许 Option<Uuid>。
use crate::application::concierge::Concierge;
use crate::common::error::Result;
use crate::infrastructure::locale;
use uuid::Uuid;

pub async fn dispatch(
    _concierge: &mut Concierge,
    _task_id: Option<Uuid>,
    new_locale: &str,
) -> Result<()> {
    locale::init(Some(new_locale));
    Ok(())
}
