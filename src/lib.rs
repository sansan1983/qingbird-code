//! eflow - Efficient Flow Agent Collaboration Framework
//!
//! A multi-layer Agent collaboration framework with zero-blocking dialogue.

rust_i18n::i18n!("locales", fallback = "en-US");

pub mod application;
pub mod capability;
pub mod cli;
pub mod common;
pub mod infrastructure;
pub mod interaction;
