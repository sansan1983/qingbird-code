/// i18n 支持的 locale 列表（与设计文档 v4.0 第 20.1 节一致）
pub const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en-US"];

/// i18n 默认 locale（设计文档 20.5：简体中文）
pub const DEFAULT_LOCALE: &str = "zh-CN";

/// 初始化 i18n
///
/// 优先级：CLI 参数（`--lang`，M13 实施） > 配置文件 `core.language` > 默认值
///
/// v1.0：仅在启动时由 `main` 调用一次。传入 `None` 时使用默认 locale。
///
/// 返回值为实际生效的 locale；使用 lifetime elision，返回与入参相同生命周期的 `&str`
/// （DEFAULT_LOCALE 是 `'static`，可被强制转成任何生命周期）。
pub fn init(config_locale: Option<&str>) -> &str {
    let locale = match config_locale {
        Some(s) if SUPPORTED_LOCALES.contains(&s) => s,
        _ => DEFAULT_LOCALE,
    };
    rust_i18n::set_locale(locale);
    locale
}
