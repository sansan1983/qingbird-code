//! stdin 协议
//!
//! 5 个 action JSON：send / end / level / lang / help
//!
//! 关键设计决策：
//! - `Send` 的 task_id 可选——GUI 不必先知道 task_id；eflow 自动生成
//! - 用 `#[serde(tag = "action")]` 做 enum 标签——JSON 直观
//! - **解析失败不退出**——stdin 网络抖动时 GUI 偶尔发坏 JSON 不该让 eflow 死

use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

use crate::application::concierge::Concierge;

use super::handlers;
use super::output::CliOutput;

#[derive(Deserialize, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum StdinCommand {
    /// 派发 task 给 Concierge（task_id 选填——GUI 不必先知道）
    Send { task_id: Option<Uuid>, task: String },
    /// 取消 task 并退出 session
    End { task_id: Uuid },
    /// 设置 risk level
    Level { task_id: Uuid, level: String },
    /// 切语言（task_id 选填——lang 不绑 task）
    Lang {
        task_id: Option<Uuid>,
        locale: String,
    },
    /// 列可用 commands
    Help,
}

/// 持续读 stdin 一行行，dispatch 到 5 个 handler
///
/// 返回 i32 exit code（0 = 正常 EOF / `end` action）
pub async fn read_loop(concierge: &mut Concierge) -> i32 {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => return 0, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue; // 空行跳过
                }

                match serde_json::from_str::<StdinCommand>(trimmed) {
                    Ok(StdinCommand::Send { task_id, task }) => {
                        let _ = handlers::send::dispatch(concierge, task_id, &task).await;
                    }
                    Ok(StdinCommand::End { task_id }) => {
                        let _ = handlers::end::dispatch(concierge, task_id).await;
                        return 0;
                    }
                    Ok(StdinCommand::Level { task_id, level }) => {
                        let _ = handlers::level::dispatch(concierge, task_id, &level).await;
                    }
                    Ok(StdinCommand::Lang { task_id, locale }) => {
                        let _ = handlers::lang::dispatch(concierge, task_id, &locale).await;
                    }
                    Ok(StdinCommand::Help) => {
                        let _ = handlers::help::dispatch(concierge).await;
                    }
                    Err(e) => {
                        // 解析失败 → stderr 报错 + **继续**读下一行
                        CliOutput::error(&format!("stdin parse failed: {e}"));
                    }
                }
            }
            Err(e) => {
                CliOutput::error(&format!("stdin read error: {e}"));
                return 2; // 系统错误
            }
        }
    }
}

use serde::Deserialize;

#[cfg(test)]
mod tests {
    use super::*;

    // 7 个 serde round-trip 测试（spec B2 §3.6）
    //
    // 注意：plan 笔误——`"0000...0001"` 解析出的是 `Uuid::from_u128(1)`，不是
    // `Uuid::nil()`。修正断言为 `from_u128(1)`。

    #[test]
    fn stdin_command_serde_send_with_task_id() {
        let json = r#"{"action": "send", "task_id": "00000000-0000-0000-0000-000000000001", "task": "test"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        match cmd {
            StdinCommand::Send { task_id, task } => {
                assert_eq!(task_id, Some(Uuid::from_u128(1)));
                assert_eq!(task, "test");
            }
            _ => panic!("expected Send"),
        }
    }

    #[test]
    fn stdin_command_serde_send_without_task_id() {
        let json = r#"{"action": "send", "task": "test"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        match cmd {
            StdinCommand::Send { task_id, task } => {
                assert_eq!(task_id, None);
                assert_eq!(task, "test");
            }
            _ => panic!("expected Send"),
        }
    }

    #[test]
    fn stdin_command_serde_end() {
        let json = r#"{"action": "end", "task_id": "00000000-0000-0000-0000-000000000001"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, StdinCommand::End { .. }));
    }

    #[test]
    fn stdin_command_serde_level() {
        let json = r#"{"action": "level", "task_id": "00000000-0000-0000-0000-000000000001", "level": "simple"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        match cmd {
            StdinCommand::Level { level, .. } => assert_eq!(level, "simple"),
            _ => panic!("expected Level"),
        }
    }

    #[test]
    fn stdin_command_serde_lang() {
        let json = r#"{"action": "lang", "locale": "en-US"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        match cmd {
            StdinCommand::Lang { locale, .. } => assert_eq!(locale, "en-US"),
            _ => panic!("expected Lang"),
        }
    }

    #[test]
    fn stdin_command_serde_help() {
        let json = r#"{"action": "help"}"#;
        let cmd: StdinCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, StdinCommand::Help));
    }

    #[test]
    fn stdin_command_invalid_action_fails() {
        let json = r#"{"action": "unknown", "x": 1}"#;
        let result: std::result::Result<StdinCommand, serde_json::Error> =
            serde_json::from_str(json);
        assert!(result.is_err());
    }
}
