/// i18n supported locales (Keep-a-Changelog & AGENTS.md §i18n).
pub const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en-US"];

/// Default locale when no CLI flag or yaml `core.language` is set.
pub const DEFAULT_LOCALE: &str = "zh-CN";

/// Initialize i18n.
///
/// The caller is expected to resolve the priority chain `--lang` CLI flag
/// \> yaml `core.language` \> `DEFAULT_LOCALE` (zh-CN) and pass the resolved
/// value here. This function validates the resolved value against
/// `SUPPORTED_LOCALES`; an unsupported value falls back to `DEFAULT_LOCALE`.
///
/// Returned value is the locale actually activated (matches what
/// `rust_i18n::set_locale` was called with), with `'static` lifetime.
pub fn init(resolved_locale: &str) -> &str {
    let locale = if SUPPORTED_LOCALES.contains(&resolved_locale) {
        resolved_locale
    } else {
        DEFAULT_LOCALE
    };
    rust_i18n::set_locale(locale);
    locale
}
