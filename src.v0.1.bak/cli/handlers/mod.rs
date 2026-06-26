//! v1.3.2 stdin 5 个 action handlers（占位）—— 真实现见 T5/T6
//!
//! 5 个 handlers：send / end / level / lang / help
//!
//! 当前每个 handler 都返回占位 `Ok(())`；T6 替换为真 dispatch。

pub mod end;
pub mod help;
pub mod lang;
pub mod level;
pub mod send;
