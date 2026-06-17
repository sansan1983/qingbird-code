//! v1.3.1 斜杠命令子系统
//!
//! 核心零硬编码命令名：每个命令 1 个 `impl SlashCommand`，
//! 通过 `CommandRegistry::register()` 注册，main.rs 启动时统一注册。
//! 加新命令 = 写 1 个 `impl` + 1 行 `register()`，**核心零修改**。
//!
//! 关键设计决策：
//! - `SlashArgs(HashMap<String, String>)` 避免 `Box<dyn Any>` 类型擦除
//! - `required_register(&[&str])` 启动时校验必需命令
//! - `HelpCmd` 构造时**捕获** registry 命令列表（避免循环依赖）

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::common::error::{EflowError, Result};

/// 斜杠命令 trait
///
/// **v1.3.1 实施后冻结**（spec B1 §10 ADR-0014 + spec B2/C 复用）：
/// - 方法签名不再破坏性变更
/// - 新增方法必须有默认实现
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// 命令名（不含斜杠），如 "model"/"level"
    fn name(&self) -> &'static str;

    /// 一行帮助（`/help` 时显示）
    fn help(&self) -> &'static str;

    /// 解析 raw 为参数（默认实现：按空格 split 成位置→token 映射）
    fn parse_args(&self, raw: &str) -> Result<SlashArgs> {
        let tokens: Vec<String> = raw.split_whitespace().map(String::from).collect();
        let mut map = HashMap::new();
        for (i, token) in tokens.into_iter().enumerate() {
            map.insert(format!("arg{i}"), token);
        }
        Ok(SlashArgs(map))
    }

    /// 执行命令（在 Concierge 主循环里调，可访问 Concierge + 状态）
    async fn execute(&self, args: SlashArgs, ctx: &mut CommandContext) -> Result<SlashOutput>;
}

/// 命令上下文（在 `execute` 调用期间持有 Concierge 借用 + router Arc 共享）
///
/// v1.3.1 deviation: router 改成 `Arc<Mutex<>>` 共享——避免 v1.3.0 Concierge 不持 router
/// 时借用冲突。execute 内需要 router 时 lock 一次。
pub struct CommandContext<'a> {
    pub concierge: &'a mut crate::application::concierge::Concierge,
    pub router: Arc<Mutex<crate::infrastructure::llm::router::LlmRouter>>,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        concierge: &'a mut crate::application::concierge::Concierge,
        router: Arc<Mutex<crate::infrastructure::llm::router::LlmRouter>>,
    ) -> Self {
        Self { concierge, router }
    }
}

/// 命令参数
#[derive(Debug, Clone, Default)]
pub struct SlashArgs(pub HashMap<String, String>);

impl SlashArgs {
    /// 单 key value 构造（最常用）
    pub fn from_kv(pairs: &[(&str, &str)]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in pairs {
            map.insert((*k).to_string(), (*v).to_string());
        }
        Self(map)
    }

    /// 取第一个参数（位置 0）
    pub fn first(&self) -> Option<&String> {
        self.0.get("arg0")
    }

    /// 取指定 key 的参数
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }
}

/// 命令输出
#[derive(Debug, Clone)]
pub enum SlashOutput {
    /// 文本输出（推到 messages 区）
    Text(String),
    /// 需要重新构造 LlmRouter（`/model` 改了 routing）
    ReloadRouter,
    /// 退出 eflow
    Shutdown,
    /// 打开子视图（`/model` 弹 SelectList）—— **临时硬编码**，v1.4 spec D 重构
    OpenSubView(Arc<crate::interaction::widgets::select_list::SelectList>),
    /// 啥也不做（`/lang` 改了 i18n 但 TUI 不需要重画）
    NoOp,
}

/// 命令注册表
pub struct CommandRegistry {
    commands: HashMap<&'static str, std::sync::Arc<dyn SlashCommand>>,
}

pub mod builtin;

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// 注册一个命令。重复 name → 第二个赢 + warn。
    pub fn register(&mut self, cmd: std::sync::Arc<dyn SlashCommand>) {
        let name = cmd.name();
        if self.commands.contains_key(name) {
            tracing::warn!("斜杠命令 {} 重复注册，第二个赢", name);
        }
        self.commands.insert(name, cmd);
    }

    /// dispatch raw（已去掉前缀 `/`）。返回 (name, parsed_args)。
    pub fn dispatch(&self, raw: &str) -> Option<(&'static str, SlashArgs)> {
        let mut parts = raw.splitn(2, char::is_whitespace);
        let name = parts.next()?.trim();
        let rest = parts.next().unwrap_or("").trim();
        let cmd = self.commands.get(name)?;
        let args = cmd.parse_args(rest).ok()?;
        Some((cmd.name(), args))
    }

    /// 取已注册命令（按 name 字母序，给 `/help` 用）
    #[must_use]
    pub fn list(&self) -> Vec<(&'static str, &'static str)> {
        let mut entries: Vec<_> = self
            .commands
            .values()
            .map(|cmd| (cmd.name(), cmd.help()))
            .collect();
        entries.sort_by_key(|(name, _)| *name);
        entries
    }

    /// 校验必需命令已注册，缺失 → Err
    pub fn required_register(&mut self, required: &[&'static str]) -> Result<()> {
        let missing: Vec<&&str> = required
            .iter()
            .filter(|name| !self.commands.contains_key(**name))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            let names: Vec<String> = missing.iter().map(|s| (*s).to_string()).collect();
            Err(EflowError::Config(format!(
                "必需斜杠命令未注册: {}",
                names.join(", ")
            )))
        }
    }

    /// 取已注册命令的 Arc 引用（execute 时用）
    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<dyn SlashCommand>> {
        self.commands.get(name)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockCmd {
        n: &'static str,
        h: &'static str,
    }

    #[async_trait]
    impl SlashCommand for MockCmd {
        fn name(&self) -> &'static str {
            self.n
        }
        fn help(&self) -> &'static str {
            self.h
        }
        async fn execute(
            &self,
            _args: SlashArgs,
            _ctx: &mut CommandContext,
        ) -> Result<SlashOutput> {
            Ok(SlashOutput::NoOp)
        }
    }

    #[test]
    fn register_and_dispatch() {
        let mut reg = CommandRegistry::new();
        reg.register(std::sync::Arc::new(MockCmd {
            n: "test",
            h: "Test command",
        }));
        let result = reg.dispatch("test arg1 arg2");
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "test");
        assert_eq!(args.0.get("arg0"), Some(&"arg1".to_string()));
        assert_eq!(args.0.get("arg1"), Some(&"arg2".to_string()));
    }

    #[test]
    fn register_duplicate_keeps_latest() {
        let mut reg = CommandRegistry::new();
        reg.register(std::sync::Arc::new(MockCmd {
            n: "test",
            h: "first",
        }));
        reg.register(std::sync::Arc::new(MockCmd {
            n: "test",
            h: "second",
        }));
        let (_, _args) = reg.dispatch("test").unwrap();
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1, "second"); // 第二个赢
    }

    #[test]
    fn dispatch_nonexistent_command_returns_none() {
        let reg = CommandRegistry::new();
        assert!(reg.dispatch("nonexistent").is_none());
    }

    #[test]
    fn parse_args_default_splits_by_whitespace() {
        struct ParseArgsMock;
        #[async_trait]
        impl SlashCommand for ParseArgsMock {
            fn name(&self) -> &'static str {
                "parse"
            }
            fn help(&self) -> &'static str {
                ""
            }
            async fn execute(
                &self,
                _args: SlashArgs,
                _ctx: &mut CommandContext,
            ) -> Result<SlashOutput> {
                Ok(SlashOutput::NoOp)
            }
        }
        let cmd = ParseArgsMock;
        let args = cmd.parse_args("a b c").unwrap();
        assert_eq!(args.0.get("arg0"), Some(&"a".to_string()));
        assert_eq!(args.0.get("arg1"), Some(&"b".to_string()));
        assert_eq!(args.0.get("arg2"), Some(&"c".to_string()));
    }

    #[test]
    fn list_returns_sorted_by_name() {
        let mut reg = CommandRegistry::new();
        reg.register(std::sync::Arc::new(MockCmd { n: "z", h: "" }));
        reg.register(std::sync::Arc::new(MockCmd { n: "a", h: "" }));
        reg.register(std::sync::Arc::new(MockCmd { n: "m", h: "" }));
        let list = reg.list();
        assert_eq!(list, vec![("a", ""), ("m", ""), ("z", "")]);
    }

    #[test]
    fn required_register_passes_when_all_present() {
        let mut reg = CommandRegistry::new();
        reg.register(std::sync::Arc::new(MockCmd { n: "a", h: "" }));
        reg.register(std::sync::Arc::new(MockCmd { n: "b", h: "" }));
        assert!(reg.required_register(&["a", "b"]).is_ok());
    }

    #[test]
    fn required_register_fails_when_missing() {
        let mut reg = CommandRegistry::new();
        reg.register(std::sync::Arc::new(MockCmd { n: "a", h: "" }));
        let result = reg.required_register(&["a", "missing"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }
}
