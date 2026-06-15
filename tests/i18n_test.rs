rust_i18n::i18n!("locales", fallback = "en-US");

use eflow::infrastructure::locale;

#[test]
fn test_default_locale_is_zh_cn() {
    assert_eq!(locale::DEFAULT_LOCALE, "zh-CN");
}

#[test]
fn test_supported_locales_contains_zh_and_en() {
    assert!(locale::SUPPORTED_LOCALES.contains(&"zh-CN"));
    assert!(locale::SUPPORTED_LOCALES.contains(&"en-US"));
}

#[test]
fn test_init_with_none_uses_default() {
    let l = locale::init(None);
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
fn test_init_with_valid_locale_uses_it() {
    let l = locale::init(Some("en-US"));
    assert_eq!(l, "en-US");
    assert_eq!(&*rust_i18n::locale(), "en-US");

    let l = locale::init(Some("zh-CN"));
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
fn test_init_with_unsupported_locale_falls_back_to_default() {
    let l = locale::init(Some("fr-FR"));
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
fn test_zh_translation_resolves() {
    locale::init(Some("zh-CN"));
    let s = rust_i18n::t!("err_profile_not_found", name = "developer");
    assert!(s.contains("developer"));
    // 中文输出应包含非 ASCII
    assert!(!s.is_ascii(), "expected non-ASCII, got: {}", s);
}

#[test]
fn test_en_translation_resolves() {
    locale::init(Some("en-US"));
    let s = rust_i18n::t!("err_profile_not_found", name = "developer");
    assert!(s.contains("developer"));
    assert!(s.contains("not found"), "got: {}", s);
}

#[test]
fn test_translations_exist_for_all_keys() {
    locale::init(Some("zh-CN"));
    assert!(!rust_i18n::t!("_system_prompt").is_empty());
    assert!(!rust_i18n::t!("err_config_load", msg = "x").is_empty());
    assert!(!rust_i18n::t!("err_http", msg = "x").is_empty());
    assert!(!rust_i18n::t!("err_no_provider", tier = "Strong").is_empty());

    locale::init(Some("en-US"));
    assert!(!rust_i18n::t!("_system_prompt").is_empty());
    assert!(!rust_i18n::t!("err_config_load", msg = "x").is_empty());
    assert!(!rust_i18n::t!("err_http", msg = "x").is_empty());
    assert!(!rust_i18n::t!("err_no_provider", tier = "Strong").is_empty());
}

#[test]
fn test_context_compressor_uses_current_locale() {
    use eflow::infrastructure::context::ContextCompressor;

    locale::init(Some("en-US"));
    let s = ContextCompressor::compress_file_content("test.rs", "a\nb\nc");
    assert!(s.0 == "a\nb\nc");
    assert!(s.1.summary.contains("test.rs"));
    assert!(s.1.summary.contains("3"));
    // 英文输出含 "lines" / "bytes"
    assert!(
        s.1.summary.contains("lines") || s.1.summary.contains("bytes"),
        "expected english output, got: {}",
        s.1.summary
    );

    locale::init(Some("zh-CN"));
    let s = ContextCompressor::compress_file_content("test.rs", "a\nb\nc");
    assert!(s.1.summary.contains("test.rs"));
    assert!(s.1.summary.contains("3"));
    // 中文输出含 "行" 或 "字节"
    assert!(
        s.1.summary.contains("行") || s.1.summary.contains("字节"),
        "expected chinese output, got: {}",
        s.1.summary
    );
}

#[test]
fn test_config_error_translates() {
    use eflow::common::error::EflowError;
    use eflow::infrastructure::config::load_config;
    use std::io::Write;

    locale::init(Some("zh-CN"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.yaml");
    let err = load_config(&path).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("未找到") || msg.contains("失败") || msg.contains("读取"));

    locale::init(Some("en-US"));
    let err = load_config(&path).unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("Failed") || msg.contains("read"),
        "expected english error, got: {}",
        msg
    );
}
