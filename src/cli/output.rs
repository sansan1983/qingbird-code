//! stdout/stderr 协议
//!
//! 关键设计决策（spec B2 §3.5）：
//! - `json` 用 println!（stdout）——**不**flush（GUI 读行前不需强制 flush）
//! - `ndjson_event` 立即 flush——**events 实时性关键**
//! - `info` / `error` 用 eprintln!（stderr）
//! - **不**直接调 tracing——tracing 自己配 tracing_subscriber 走 stderr

use serde::Serialize;

pub struct CliOutput;

impl CliOutput {
    /// 输出 JSON 对象到 stdout（**永远只走 stdout**）
    pub fn json<T: Serialize>(value: &T) -> crate::common::error::Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| crate::common::error::EflowError::Serialization(e.to_string()))?;
        println!("{}", json);
        Ok(())
    }

    /// 输出一行 NDJSON（events 流用）——立即 flush
    pub fn ndjson_event<T: Serialize>(value: &T) -> crate::common::error::Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| crate::common::error::EflowError::Serialization(e.to_string()))?;
        println!("{}", json);
        // 立即 flush，让 GUI 实时收到
        use std::io::Write;
        std::io::stdout().flush().ok();
        Ok(())
    }

    /// 输出人类可读消息到 stderr
    pub fn info(msg: &str) {
        eprintln!("{}", msg);
    }

    /// 输出错误消息到 stderr
    pub fn error(msg: &str) {
        eprintln!("error: {}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // v1.3.2 T2: stdout/stderr 协议单元测试
    //
    // 简化测试：只验证 4 个方法能调（完整 capture 行为由 tests/gui_smoke_test.py
    // 在真实 qingbird 进程上验证——这里调 stdout/stderr 没法在 unit test 干净 capture）

    #[test]
    fn json_outputs_to_stdout_with_newline() {
        CliOutput::json(&json!({"test": 1})).unwrap();
    }

    #[test]
    fn ndjson_event_flushes() {
        CliOutput::ndjson_event(&json!({"event": "test"})).unwrap();
    }

    #[test]
    fn info_writes_to_stderr() {
        CliOutput::info("test info");
    }

    #[test]
    fn error_writes_to_stderr() {
        CliOutput::error("test error");
    }
}
