use qbird_code_infra::locale::{DEFAULT_LOCALE, SUPPORTED_LOCALES, init};

#[test]
fn test_init_zh_cn_activates() {
    let result = init("zh-CN");
    assert_eq!(result, "zh-CN");
    assert!(SUPPORTED_LOCALES.contains(&result));
}

#[test]
fn test_init_en_us_activates() {
    let result = init("en-US");
    assert_eq!(result, "en-US");
    assert!(SUPPORTED_LOCALES.contains(&result));
}

#[test]
fn test_init_unsupported_falls_back_to_default() {
    let result = init("de-DE");
    assert_eq!(result, DEFAULT_LOCALE);
}

#[test]
fn test_init_empty_falls_back_to_default() {
    let result = init("");
    assert_eq!(result, DEFAULT_LOCALE);
}

#[test]
fn test_init_default_locale_is_zh_cn() {
    assert_eq!(DEFAULT_LOCALE, "zh-CN");
}

#[test]
fn test_supported_locales_exact_set() {
    assert_eq!(SUPPORTED_LOCALES, &["zh-CN", "en-US"]);
}
