rust_i18n::i18n!("locales", fallback = "en-US");

use eflow::capability::tools::{Tool, ToolDefinition, ToolOutput, ToolRegistry};
use eflow::common::error::EflowError;
use eflow::common::types::RiskLevel;
use eflow::infrastructure::locale;
use std::fs;
use std::sync::Arc;

// 切换到中文让中文输出断言能通过
// locale setup moved into individual tests

fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path
}

// ========== ToolRegistry 测试 ==========

#[test]
fn test_registry_new_is_empty() {
    let reg = ToolRegistry::new();
    assert!(reg.get("any").is_none());
    assert!(reg.definitions().is_empty());
}

#[test]
fn test_registry_register_and_get() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(eflow::capability::tools::file::ReadFileTool));
    let got = reg.get("read_file");
    assert!(got.is_some());
    assert_eq!(got.unwrap().definition().name, "read_file");
}

#[test]
fn test_registry_definitions_returns_all() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(eflow::capability::tools::file::ReadFileTool));
    reg.register(Arc::new(eflow::capability::tools::file::WriteFileTool));
    let defs = reg.definitions();
    assert_eq!(defs.len(), 2);
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
}

#[tokio::test]
async fn test_registry_execute_unknown_tool_returns_error() {
    let reg = ToolRegistry::new();
    let err = reg
        .execute("nonexistent", serde_json::json!({}))
        .await
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("nonexistent") || msg.contains("未找到"));
}

// ========== L3 工具测试 ==========

struct L3StubTool;

#[async_trait::async_trait]
impl Tool for L3StubTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "l3_stub".into(),
            description: "L3 risk stub for testing".into(),
            parameters: serde_json::json!({}),
            risk_level: RiskLevel::L3,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolOutput, EflowError> {
        Ok(ToolOutput {
            success: true,
            content: "should not run".into(),
            metadata: None,
        })
    }
}

#[tokio::test]
async fn test_l3_tool_is_rejected() {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(L3StubTool));
    let err = reg
        .execute("l3_stub", serde_json::json!({}))
        .await
        .unwrap_err();
    matches!(err, EflowError::RiskEscalated { .. });
}

// ========== ReadFileTool ==========

#[tokio::test]
async fn test_read_file_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "hello.txt", "line1\nline2\nline3");

    let tool = eflow::capability::tools::file::ReadFileTool;
    let out = tool
        .execute(serde_json::json!({"path": path.to_str().unwrap()}))
        .await
        .unwrap();
    assert!(out.success);
    assert!(out.content.contains("line1"));
    assert!(out.content.contains("line3"));
    assert!(out.content.contains("3"));
    let meta = out.metadata.unwrap();
    assert_eq!(meta["lines"], 3);
}

#[tokio::test]
async fn test_read_file_missing_path_param() {
    let tool = eflow::capability::tools::file::ReadFileTool;
    let err = tool.execute(serde_json::json!({})).await.unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

#[tokio::test]
async fn test_read_file_not_found() {
    let tool = eflow::capability::tools::file::ReadFileTool;
    let err = tool
        .execute(serde_json::json!({"path": "/nonexistent/abc/xyz.txt"}))
        .await
        .unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

// ========== WriteFileTool ==========

#[tokio::test]
async fn test_write_file_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.txt");

    let tool = eflow::capability::tools::file::WriteFileTool;
    let out = tool
        .execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "hello world"
        }))
        .await
        .unwrap();
    assert!(out.success);
    let written = fs::read_to_string(&path).unwrap();
    assert_eq!(written, "hello world");
    let meta = out.metadata.unwrap();
    assert_eq!(meta["bytes_written"], 11);
}

#[tokio::test]
async fn test_write_file_missing_content_param() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.txt");
    let tool = eflow::capability::tools::file::WriteFileTool;
    let err = tool
        .execute(serde_json::json!({"path": path.to_str().unwrap()}))
        .await
        .unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

// ========== ExecuteCommandTool（跨平台） ==========

#[tokio::test]
async fn test_execute_command_echo_hello() {
    // 用 `true` 作为跨平台 smoke test（Windows/Unix 都有 `true`，退出码 0）
    let tool = eflow::capability::tools::command::ExecuteCommandTool;
    let cmd = if cfg!(windows) { "cmd" } else { "true" };
    let args: Vec<&str> = if cfg!(windows) {
        vec!["/c", "echo", "hello"]
    } else {
        vec![]
    };

    let out = tool
        .execute(serde_json::json!({
            "command": cmd,
            "args": args
        }))
        .await
        .unwrap();
    assert!(out.success);
    let meta = out.metadata.unwrap();
    assert_eq!(meta["exit_code"], 0);
}

#[tokio::test]
async fn test_execute_command_captures_exit_code() {
    let tool = eflow::capability::tools::command::ExecuteCommandTool;
    // 跨平台：让进程成功启动但返回非零退出码
    let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("cmd", vec!["/c", "exit", "1"])
    } else {
        ("false", vec![])
    };
    let out = tool
        .execute(serde_json::json!({
            "command": cmd,
            "args": args
        }))
        .await
        .unwrap();
    assert!(!out.success, "expected non-zero exit, got success=true");
    let meta = out.metadata.unwrap();
    assert_eq!(meta["exit_code"], 1);
}

#[tokio::test]
async fn test_execute_command_missing_command_param() {
    let tool = eflow::capability::tools::command::ExecuteCommandTool;
    let err = tool.execute(serde_json::json!({})).await.unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

// ========== SearchCodeTool（Rust 原生） ==========

#[tokio::test]
async fn test_search_code_finds_pattern() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "a.rs",
        "fn foo() {}\nfn bar() {}\nfn baz() {}\n",
    );
    write_file(dir.path(), "b.toml", "name = \"x\"\n");

    let tool = eflow::capability::tools::search::SearchCodeTool;
    let out = tool
        .execute(serde_json::json!({
            "pattern": "fn (foo|bar)",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();
    assert!(out.success);
    let meta = out.metadata.clone().unwrap();
    assert_eq!(meta["matches"], 2);
    assert!(out.content.contains("fn foo()"));
    assert!(out.content.contains("fn bar()"));
    assert!(!out.content.contains("fn baz()"));
}

#[tokio::test]
#[serial_test::serial]
async fn test_search_code_no_match() {
    locale::init(Some("zh-CN"));
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "a.rs", "fn foo() {}\n");

    let tool = eflow::capability::tools::search::SearchCodeTool;
    let out = tool
        .execute(serde_json::json!({
            "pattern": "nonexistent_pattern_xyz",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();
    assert!(out.success);
    let meta = out.metadata.unwrap();
    assert_eq!(meta["matches"], 0);
    assert!(out.content.contains("未找到") || out.content.contains("匹配"));
}

#[tokio::test]
async fn test_search_code_filters_by_file_type() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "a.rs", "TARGET\n");
    write_file(dir.path(), "b.txt", "TARGET\n");

    let tool = eflow::capability::tools::search::SearchCodeTool;
    let out = tool
        .execute(serde_json::json!({
            "pattern": "TARGET",
            "path": dir.path().to_str().unwrap(),
            "file_types": "*.rs"
        }))
        .await
        .unwrap();
    let meta = out.metadata.unwrap();
    assert_eq!(meta["matches"], 1);
    assert!(out.content.contains("a.rs"));
    assert!(!out.content.contains("b.txt"));
}

#[tokio::test]
async fn test_search_code_recursive_into_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    write_file(dir.path(), "src/lib.rs", "DEEP_MATCH\n");
    let sub = dir.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    write_file(&sub, "deep.rs", "DEEP_MATCH here too\n");

    let tool = eflow::capability::tools::search::SearchCodeTool;
    let out = tool
        .execute(serde_json::json!({
            "pattern": "DEEP_MATCH",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();
    let meta = out.metadata.unwrap();
    assert_eq!(meta["matches"], 2);
}

#[tokio::test]
async fn test_search_code_skips_large_files() {
    let dir = tempfile::tempdir().unwrap();
    // 写 2MB 文件 — 超过 1 MiB 限制
    let big_content = "x".repeat(2 * 1024 * 1024);
    write_file(dir.path(), "big.rs", &big_content);
    write_file(dir.path(), "small.rs", "SMALL_MATCH\n");

    let tool = eflow::capability::tools::search::SearchCodeTool;
    let out = tool
        .execute(serde_json::json!({
            "pattern": "SMALL_MATCH",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();
    let meta = out.metadata.unwrap();
    assert_eq!(meta["matches"], 1);
    assert_eq!(meta["files_scanned"], 1); // big.rs 被跳过
}

#[tokio::test]
async fn test_search_code_invalid_regex() {
    let tool = eflow::capability::tools::search::SearchCodeTool;
    let err = tool
        .execute(serde_json::json!({"pattern": "[unclosed"}))
        .await
        .unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

#[tokio::test]
async fn test_search_code_invalid_path() {
    let tool = eflow::capability::tools::search::SearchCodeTool;
    let err = tool
        .execute(serde_json::json!({
            "pattern": "x",
            "path": "/nonexistent/dir/xyz"
        }))
        .await
        .unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

#[tokio::test]
async fn test_search_code_missing_pattern() {
    let tool = eflow::capability::tools::search::SearchCodeTool;
    let err = tool.execute(serde_json::json!({})).await.unwrap_err();
    assert!(matches!(err, EflowError::Tool(_)));
}

// ========== i18n ==========

#[tokio::test]
async fn test_tool_error_translates() {
    locale::init(Some("en-US"));
    let reg = ToolRegistry::new();
    let err = reg
        .execute("ghost", serde_json::json!({}))
        .await
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("ghost") || msg.contains("not found"),
        "got: {}",
        msg
    );

    locale::init(Some("zh-CN"));
    let err = reg
        .execute("ghost", serde_json::json!({}))
        .await
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("ghost") || msg.contains("未找到"),
        "got: {}",
        msg
    );
}
