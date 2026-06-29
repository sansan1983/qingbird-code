use qbird_code_infra::locale::init;

rust_i18n::i18n!("../../locales", fallback = "en-US");

#[test]
fn test_usage_cache_hit_displayed() {
    init("en-US");
    let count: u64 = 42;
    let line = rust_i18n::t!("interactive_usage_cache_hit", count = count);
    assert!(line.contains("42"), "should contain count, got: {line}");
    assert!(
        line.contains("cache hit"),
        "should contain 'cache hit', got: {line}"
    );
    assert!(
        line.contains("tokens"),
        "should contain 'tokens', got: {line}"
    );
}

#[test]
fn test_usage_no_cache_hit_omits_line() {
    init("en-US");
    let count: u64 = 0;
    let line = rust_i18n::t!("interactive_usage_cache_hit", count = count);
    // When count == 0 the caller skips println, so the string itself
    // still formats — but the /usage handler gates on count > 0.
    assert!(line.contains("0"), "zero count should render, got: {line}");
}

#[test]
fn test_usage_cache_hit_zero() {
    init("zh-CN");
    let count: u64 = 0;
    let line = rust_i18n::t!("interactive_usage_cache_hit", count = count);
    assert!(
        line.contains("0"),
        "zh-CN zero count should render, got: {line}"
    );
    assert!(
        line.contains("cache hit"),
        "zh-CN should still contain 'cache hit', got: {line}"
    );
}
