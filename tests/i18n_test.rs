rust_i18n::i18n!("locales", fallback = "en-US");

use qingbird_code::infrastructure::locale;

#[test]
#[serial_test::serial]
fn test_default_locale_is_zh_cn() {
    assert_eq!(locale::DEFAULT_LOCALE, "zh-CN");
}

#[test]
#[serial_test::serial]
fn test_supported_locales_contains_zh_and_en() {
    assert!(locale::SUPPORTED_LOCALES.contains(&"zh-CN"));
    assert!(locale::SUPPORTED_LOCALES.contains(&"en-US"));
}

#[test]
#[serial_test::serial]
fn test_init_with_none_uses_default() {
    let l = locale::init(None);
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
#[serial_test::serial]
fn test_init_with_valid_locale_uses_it() {
    let l = locale::init(Some("en-US"));
    assert_eq!(l, "en-US");
    assert_eq!(&*rust_i18n::locale(), "en-US");

    let l = locale::init(Some("zh-CN"));
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
#[serial_test::serial]
fn test_init_with_unsupported_locale_falls_back_to_default() {
    let l = locale::init(Some("fr-FR"));
    assert_eq!(l, "zh-CN");
    assert_eq!(&*rust_i18n::locale(), "zh-CN");
}

#[test]
#[serial_test::serial]
fn test_zh_translation_resolves() {
    locale::init(Some("zh-CN"));
    let s = rust_i18n::t!("err_profile_not_found", name = "developer");
    assert!(s.contains("developer"));
    // 中文输出应包含非 ASCII
    assert!(!s.is_ascii(), "expected non-ASCII, got: {}", s);
}

#[test]
#[serial_test::serial]
fn test_en_translation_resolves() {
    locale::init(Some("en-US"));
    let s = rust_i18n::t!("err_profile_not_found", name = "developer");
    assert!(s.contains("developer"));
    assert!(s.contains("not found"), "got: {}", s);
}

#[test]
#[serial_test::serial]
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
#[serial_test::serial]
fn test_context_compressor_uses_current_locale() {
    use qingbird_code::infrastructure::context::ContextCompressor;

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
#[serial_test::serial]
fn test_config_error_translates() {
    use qingbird_code::infrastructure::config::load_config;

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

#[test]
#[serial_test::serial]
fn new_llm_error_keys_exist_in_both_locales() {
    // v1.1 Task A6: 验证 A5 新增/复用的 LLM 错误键在双 locale 都存在
    // （plan 写 `(*key).into()` 在 rust_i18n v3 触发 ambiguous From<&str>，
    //  改用显式 3 个 t!() 调用 — 等价覆盖，避免生命周期推断坑）
    let zh1 = rust_i18n::t!("err_all_providers_limited", count = 5);
    let en1 = rust_i18n::t!("err_all_providers_limited", locale = "en-US", count = 5);
    let zh2 = rust_i18n::t!("err_no_fallback");
    let en2 = rust_i18n::t!("err_no_fallback", locale = "en-US");
    let zh3 = rust_i18n::t!("err_tier_degrade", from = "strong", to = "medium");
    let en3 = rust_i18n::t!(
        "err_tier_degrade",
        locale = "en-US",
        from = "strong",
        to = "medium"
    );
    assert!(!zh1.is_empty(), "zh err_all_providers_limited missing");
    assert!(!en1.is_empty(), "en err_all_providers_limited missing");
    assert!(!zh2.is_empty(), "zh err_no_fallback missing");
    assert!(!en2.is_empty(), "en err_no_fallback missing");
    assert!(!zh3.is_empty(), "zh err_tier_degrade missing");
    assert!(!en3.is_empty(), "en err_tier_degrade missing");
}
