//! v1.3.1 通用 widget 集合
//!
//! SelectList 是向导和运行时斜杠命令共用的选择 UI（`/model` 切模型也是 SelectList）。

pub mod select_list;
pub use select_list::{SelectAction, SelectItem, SelectItemSource, SelectList};
