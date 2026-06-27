use std::sync::Arc;

use qbird_code_models::RiskLevel;
use qbird_code_tools::Tool;
use qbird_code_tools::{
    ExecuteCommandTool, GlobTool, ReadFileTool, SearchCodeTool, ToolRegistry, WebFetchTool,
    WriteFileTool, glob_match,
};

// ===== ToolRegistry =====

#[test]
fn test_registry_new_is_empty() {
    let r = ToolRegistry::new();
    assert!(r.definitions().is_empty());
}

#[test]
fn test_registry_register_and_get() {
    let mut r = ToolRegistry::new();
    r.register(Arc::new(ReadFileTool));
    assert!(r.get("read_file").is_some());
    assert!(r.get("nonexistent").is_none());
}

#[test]
fn test_registry_definitions_returns_all() {
    let mut r = ToolRegistry::new();
    r.register(Arc::new(ReadFileTool));
    r.register(Arc::new(WriteFileTool));
    r.register(Arc::new(ExecuteCommandTool));
    r.register(Arc::new(SearchCodeTool));
    r.register(Arc::new(GlobTool));
    r.register(Arc::new(WebFetchTool));
    let defs = r.definitions();
    assert_eq!(defs.len(), 6);
    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"execute_command"));
    assert!(names.contains(&"search_code"));
    assert!(names.contains(&"glob"));
    assert!(names.contains(&"web_fetch"));
}

// ===== Risk levels =====

#[test]
fn test_tool_risk_levels() {
    assert_eq!(ReadFileTool.definition().risk_level, RiskLevel::L0);
    assert_eq!(WriteFileTool.definition().risk_level, RiskLevel::L1);
    assert_eq!(ExecuteCommandTool.definition().risk_level, RiskLevel::L2);
    assert_eq!(SearchCodeTool.definition().risk_level, RiskLevel::L0);
    assert_eq!(GlobTool.definition().risk_level, RiskLevel::L0);
    assert_eq!(WebFetchTool.definition().risk_level, RiskLevel::L0);
}

// ===== Tool definitions =====

#[test]
fn test_read_file_definition_has_path_param() {
    let def = ReadFileTool.definition();
    assert_eq!(def.name, "read_file");
    let params = def.parameters.as_object().unwrap();
    assert!(params["properties"]["path"].is_object());
    assert!(
        params["required"]
            .as_array()
            .unwrap()
            .contains(&"path".into())
    );
}

#[test]
fn test_write_file_definition_has_path_and_content() {
    let def = WriteFileTool.definition();
    assert_eq!(def.name, "write_file");
    let params = def.parameters.as_object().unwrap();
    let required: Vec<&str> = params["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"path"));
    assert!(required.contains(&"content"));
}

#[test]
fn test_search_code_definition_has_pattern() {
    let def = SearchCodeTool.definition();
    assert_eq!(def.name, "search_code");
    let params = def.parameters.as_object().unwrap();
    let required: Vec<&str> = params["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"pattern"));
}

// ===== ReadFileTool =====

#[tokio::test]
async fn test_read_file_missing_param() {
    let result = ReadFileTool.execute(serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_nonexistent() {
    let result = ReadFileTool
        .execute(serde_json::json!({"path": "/tmp/nonexistent_qbird_test_file_xx"}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_success() {
    let dir = std::env::temp_dir().join("qbird_test_read");
    let _ = std::fs::create_dir_all(&dir);
    let filepath = dir.join("test.txt");
    std::fs::write(&filepath, "hello world").unwrap();

    let result = ReadFileTool
        .execute(serde_json::json!({"path": filepath.to_str().unwrap()}))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.content.contains("hello world"));

    let _ = std::fs::remove_dir_all(&dir);
}

// ===== WriteFileTool =====

#[tokio::test]
async fn test_write_file_missing_params() {
    let r = WriteFileTool.execute(serde_json::json!({})).await;
    assert!(r.is_err());

    let r = WriteFileTool
        .execute(serde_json::json!({"path": "/tmp/x"}))
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_write_file_success() {
    let dir = std::env::temp_dir().join("qbird_test_write");
    let _ = std::fs::create_dir_all(&dir);
    let filepath = dir.join("out.txt");

    let result = WriteFileTool
        .execute(serde_json::json!({
            "path": filepath.to_str().unwrap(),
            "content": "test content 123"
        }))
        .await;
    assert!(result.is_ok());

    let content = std::fs::read_to_string(&filepath).unwrap();
    assert_eq!(content, "test content 123");

    let _ = std::fs::remove_dir_all(&dir);
}

// ===== ExecuteCommandTool =====

#[tokio::test]
async fn test_execute_command_missing_param() {
    let r = ExecuteCommandTool.execute(serde_json::json!({})).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_execute_command_echo() {
    // Platform-neutral: use "cmd /c echo" on Windows, "echo" on Unix
    let cmd = if cfg!(windows) { "cmd" } else { "echo" };
    let args = if cfg!(windows) {
        vec!["/C", "echo", "hello"]
    } else {
        vec!["hello"]
    };

    let result = ExecuteCommandTool
        .execute(serde_json::json!({
            "command": cmd,
            "args": args
        }))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.content.contains("hello"));
}

// ===== SearchCodeTool =====

#[tokio::test]
async fn test_search_code_missing_pattern() {
    let r = SearchCodeTool.execute(serde_json::json!({})).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_search_code_no_match() {
    let r = SearchCodeTool
        .execute(serde_json::json!({"pattern": "ZZ_THIS_SHOULD_NOT_MATCH_ZZ"}))
        .await;
    assert!(r.is_ok());
    let output = r.unwrap();
    assert!(output.success);
}

#[tokio::test]
async fn test_search_code_invalid_regex() {
    let r = SearchCodeTool
        .execute(serde_json::json!({"pattern": "[invalid"}))
        .await;
    assert!(r.is_err());
}

// ===== Registry execute with risk check =====

#[tokio::test]
async fn test_registry_execute_unknown_tool() {
    let r = ToolRegistry::new();
    let id = uuid::Uuid::new_v4();
    let result = r.execute("nonexistent", serde_json::json!({}), id).await;
    assert!(result.is_err());
}

// ===== Default trait =====

#[test]
fn test_registry_default() {
    let r: ToolRegistry = Default::default();
    assert!(r.definitions().is_empty());
}

// ===== GlobTool =====

#[test]
fn test_glob_match_success() {
    assert!(glob_match("*.rs", "lib.rs"));
    assert!(glob_match("**/*.rs", "src/lib.rs"));
    assert!(glob_match("foo?", "food"));
    assert!(glob_match("[abc]*", "apple"));
}

#[test]
fn test_glob_match_no_match() {
    assert!(!glob_match("*.rs", "lib.txt"));
    assert!(!glob_match("**/*.rs", "src/lib.txt"));
}

#[test]
fn test_glob_match_regex_meta_chars() {
    assert!(glob_match("foo(1).txt", "foo(1).txt"));
    assert!(glob_match("foo+bar.txt", "foo+bar.txt"));
    assert!(glob_match("foo^bar.txt", "foo^bar.txt"));
    assert!(glob_match("foo$bar.txt", "foo$bar.txt"));
    assert!(glob_match("foo|bar.txt", "foo|bar.txt"));
    assert!(glob_match("foo\\bar.txt", "foo\\bar.txt"));
}

#[test]
fn test_glob_match_braces_are_literal() {
    assert!(glob_match("foo{1}.txt", "foo{1}.txt"));
}

#[test]
fn test_glob_match_char_class() {
    assert!(glob_match("foo[1].txt", "foo1.txt"));
    assert!(glob_match("foo[!1].txt", "foo2.txt"));
}

#[test]
fn test_glob_match_invalid_pattern_does_not_panic() {
    assert!(!glob_match("[invalid", "anything"));
}

#[tokio::test]
async fn test_glob_missing_param() {
    let r = GlobTool.execute(serde_json::json!({})).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_glob_invalid_path() {
    let r = GlobTool
        .execute(serde_json::json!({
            "pattern": "*.rs",
            "path": "/nonexistent_qbird_test_path_xyz"
        }))
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_glob_success() {
    let dir = std::env::temp_dir().join("qbird_test_glob");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("test.rs"), "").unwrap();
    std::fs::write(dir.join("test.txt"), "").unwrap();

    let result = GlobTool
        .execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.to_str().unwrap()
        }))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.content.contains("test.rs"));
    assert!(!output.content.contains("test.txt"));
    assert_eq!(output.metadata.as_ref().unwrap()["matches"], 1);
    assert_eq!(output.metadata.as_ref().unwrap()["truncated"], false);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_glob_no_match() {
    let dir = std::env::temp_dir().join("qbird_test_glob_nomatch");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("test.txt"), "").unwrap();

    let result = GlobTool
        .execute(serde_json::json!({
            "pattern": "*.rs",
            "path": dir.to_str().unwrap()
        }))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert_eq!(output.metadata.as_ref().unwrap()["matches"], 0);
    assert_eq!(output.metadata.as_ref().unwrap()["truncated"], false);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_glob_double_star_matches_subdirs() {
    let dir = std::env::temp_dir().join("qbird_test_glob_ds");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("root.rs"), "").unwrap();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub").join("nested.rs"), "").unwrap();

    let result = GlobTool
        .execute(serde_json::json!({
            "pattern": "**/*.rs",
            "path": dir.to_str().unwrap()
        }))
        .await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.success);
    assert!(output.content.contains("nested.rs"));

    let _ = std::fs::remove_dir_all(&dir);
}

// ===== WebFetchTool =====

#[test]
fn test_web_fetch_definition() {
    let def = WebFetchTool.definition();
    assert_eq!(def.name, "web_fetch");
    let params = def.parameters.as_object().unwrap();
    assert!(params["properties"]["url"].is_object());
    let required: Vec<&str> = params["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"url"));
}

#[tokio::test]
async fn test_web_fetch_missing_url() {
    let result = WebFetchTool.execute(serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_web_fetch_invalid_url() {
    let result = WebFetchTool
        .execute(serde_json::json!({"url": "not a valid url://"}))
        .await;
    assert!(result.is_err());
}
