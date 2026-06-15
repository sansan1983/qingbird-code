rust_i18n::i18n!("locales", fallback = "en-US");

use clap::Parser;
use eflow::interaction::cli::Cli;

#[test]
fn cli_default_has_no_flags_set() {
    let c = Cli::parse_from(["eflow"]);
    assert!(c.execute.is_none());
    assert!(!c.show_config);
    assert!(!c.list_profiles);
    assert!(c.lang.is_none());
}

#[test]
fn cli_parses_execute_short_flag() {
    let c = Cli::parse_from(["eflow", "-e", "read README"]);
    assert_eq!(c.execute.as_deref(), Some("read README"));
}

#[test]
fn cli_parses_show_config_long_flag() {
    let c = Cli::parse_from(["eflow", "--show-config"]);
    assert!(c.show_config);
}

#[test]
fn cli_parses_list_profiles_long_flag() {
    let c = Cli::parse_from(["eflow", "--list-profiles"]);
    assert!(c.list_profiles);
}

#[test]
fn cli_parses_lang_long_flag() {
    let c = Cli::parse_from(["eflow", "--lang", "en-US"]);
    assert_eq!(c.lang.as_deref(), Some("en-US"));
}

#[test]
fn cli_combined_flags() {
    let c = Cli::parse_from([
        "eflow",
        "--lang",
        "zh-CN",
        "--list-profiles",
        "--show-config",
    ]);
    assert_eq!(c.lang.as_deref(), Some("zh-CN"));
    assert!(c.list_profiles);
    assert!(c.show_config);
}
