rust_i18n::i18n!("locales", fallback = "en-US");

use chrono::Utc;
use eflow::common::types::ActionRecord;
use eflow::infrastructure::context::{ContextCompressor, ContextRef};
use eflow::infrastructure::locale;

// 切换到中文，使 compressor 输出的中文字符串可被断言
// （每个集成测试 binary 是独立进程，全局 locale 互不干扰）
// locale setup moved into individual tests

fn record(tool: &str, success: bool, summary: &str) -> ActionRecord {
    ActionRecord {
        timestamp: Utc::now(),
        action: format!("do_{}", tool),
        tool: tool.into(),
        success,
        summary: summary.into(),
    }
}

#[test]
fn test_context_ref_new_and_format() {
    let r = ContextRef::new("a file".into(), "file:foo.rs".into(), 42);
    assert!(!r.ref_id.is_nil());
    assert_eq!(r.token_cost_if_included, 42);
    let formatted = r.format_for_context();
    assert!(formatted.starts_with("[ref:"));
    assert!(formatted.contains("a file"));
    // short id is 8 chars + "[ref:" + "] "
    assert!(formatted.len() < 50);
}

#[test]
fn test_context_ref_short_id_is_eight_chars() {
    let r = ContextRef::new("x".into(), "k".into(), 0);
    let formatted = r.format_for_context();
    let id_part = formatted
        .strip_prefix("[ref:")
        .and_then(|s| s.split(']').next())
        .unwrap();
    assert_eq!(id_part.len(), 8);
}

#[test]
#[serial_test::serial]
fn test_compress_action_log_empty() {
    locale::init(Some("zh-CN"));
    assert_eq!(ContextCompressor::compress_action_log(&[]), "无操作");
}

#[test]
#[serial_test::serial]
fn test_compress_action_log_renders_status_and_tool() {
    locale::init(Some("zh-CN"));
    let logs = vec![
        record("read_file", true, "ok"),
        record("search", false, "no match"),
    ];
    let out = ContextCompressor::compress_action_log(&logs);
    assert!(out.contains("✓"));
    assert!(out.contains("✗"));
    assert!(out.contains("read_file"));
    assert!(out.contains("search"));
    assert!(out.contains("ok"));
}

#[test]
fn test_compress_action_log_truncates_long_summary() {
    let long = "x".repeat(500);
    let logs = vec![record("t", true, &long)];
    let out = ContextCompressor::compress_action_log(&logs);
    // 500 chars truncated to 100, plus the prefix template
    assert!(out.len() < 200);
}

#[test]
#[serial_test::serial]
fn test_compress_file_content_returns_preview_and_ref() {
    locale::init(Some("zh-CN"));
    let content = "line1\nline2\nline3\nline4\nline5";
    let (preview, ctx_ref) = ContextCompressor::compress_file_content("foo.rs", content);
    assert_eq!(preview, "line1\nline2\nline3");
    assert!(ctx_ref.summary.contains("foo.rs"));
    assert!(ctx_ref.summary.contains("5行"));
    assert!(ctx_ref.summary.contains("29字节"));
    assert_eq!(ctx_ref.storage_key, "file:foo.rs");
    assert!(ctx_ref.token_cost_if_included > 0);
}

#[test]
fn test_compress_error_keeps_first_line() {
    let err = "Permission denied\n  at line 5\n  at line 10";
    let out = ContextCompressor::compress_error(err);
    assert!(out.contains("Permission denied"));
    assert!(!out.contains("line 5"));
}

#[test]
fn test_summarize_conversation_short_returns_as_is() {
    let msgs = vec!["hi".into(), "hello".into()];
    let out = ContextCompressor::summarize_conversation(&msgs, 1000);
    assert_eq!(out, "hi\nhello");
}

#[test]
#[serial_test::serial]
fn test_summarize_conversation_long_keeps_ends_and_skips_middle() {
    locale::init(Some("zh-CN"));
    let msgs: Vec<String> = (0..10).map(|i| format!("msg {}", i)).collect();
    let out = ContextCompressor::summarize_conversation(&msgs, 1000);
    assert!(out.contains("msg 0"));
    assert!(out.contains("msg 9"));
    assert!(out.contains("8 轮对话省略"));
    assert!(!out.contains("msg 5"));
}

#[test]
fn test_summarize_conversation_truncates_at_max_len() {
    let msgs: Vec<String> = (0..20)
        .map(|i| "x".repeat(50).replace('x', &format!("m{}", i)))
        .collect();
    let out = ContextCompressor::summarize_conversation(&msgs, 30);
    assert!(out.len() <= 33); // 30 + "..."
    assert!(out.ends_with("..."));
}

#[test]
fn test_summarize_conversation_does_not_panic_on_multibyte() {
    // 中文每个 char 占 3 byte，max_summary_len=5 处于 char 边界（1.67 个），
    // 旧实现 `&summary[..5]` 会 panic（v1.0.3 修真 bug B1）。
    let msgs: Vec<String> = (0..20)
        .map(|i| format!("中文字符串{}{}", "中".repeat(50), i))
        .collect();
    let out = ContextCompressor::summarize_conversation(&msgs, 5);
    assert!(
        out.ends_with("..."),
        "expected '...' suffix, got: {:?}",
        out
    );
    // 5 个 char ≈ 15 bytes (UTF-8) + "..."(3 bytes) = ≤ 18 bytes
    assert!(out.len() <= 18 + 3, "out too long: {} bytes", out.len());
}

#[test]
fn test_estimate_tokens_ceiling_division() {
    // 4 chars ≈ 1 token
    assert_eq!(ContextCompressor::estimate_tokens(""), 0);
    assert_eq!(ContextCompressor::estimate_tokens("abcd"), 1);
    assert_eq!(ContextCompressor::estimate_tokens("abcde"), 2); // 5/4 = 1.25 → 2
    assert_eq!(ContextCompressor::estimate_tokens("abcdefgh"), 2);
}

#[test]
fn test_needs_compression_triggers_above_80_percent() {
    assert!(!ContextCompressor::needs_compression(80, 100));
    assert!(!ContextCompressor::needs_compression(80, 100));
    assert!(ContextCompressor::needs_compression(81, 100));
    assert!(ContextCompressor::needs_compression(100, 100));
    assert!(!ContextCompressor::needs_compression(0, 0));
}
