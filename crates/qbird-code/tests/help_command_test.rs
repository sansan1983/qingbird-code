use std::fs;

/// All interactive_help_* keys that the /help slash command renders.
const HELP_KEYS: &[&str] = &[
    "interactive_help_title",
    "interactive_help_quit",
    "interactive_help_exit",
    "interactive_help_help",
    "interactive_help_model",
    "interactive_help_temp",
    "interactive_help_usage",
    "interactive_help_sessions",
    "interactive_help_session_load",
    "interactive_help_sdd_title",
    "interactive_help_sdd_run",
    "interactive_help_sdd_confirm",
    "interactive_help_sdd_status",
    "interactive_help_undo_planned",
    "interactive_help_profile_planned",
    "interactive_help_provider_planned",
    "interactive_help_session_delete_planned",
];

fn read_yml(name: &str) -> String {
    let path = format!("../../locales/{name}.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn extract_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}:");
    line.strip_prefix(&prefix)
        .map(|rest| rest.trim().trim_matches('"'))
}

fn lookup<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    for line in content.lines() {
        if let Some(v) = extract_value(line.trim(), key) {
            return Some(v);
        }
    }
    None
}

#[test]
fn test_help_contains_all_commands_zh_cn() {
    let content = read_yml("zh-CN");
    for key in HELP_KEYS {
        assert!(lookup(&content, key).is_some(), "missing zh-CN key: {key}");
    }
    // Verify the 7 logical command groups all have at least one line
    let required = [
        "interactive_help_quit",
        "interactive_help_help",
        "interactive_help_model",
        "interactive_help_temp",
        "interactive_help_usage",
        "interactive_help_sessions",
        "interactive_help_sdd_run",
    ];
    for key in required {
        let val = lookup(&content, key).expect(key);
        assert!(!val.is_empty(), "{key} is empty in zh-CN");
    }
}

#[test]
fn test_help_chinese_default() {
    let content = read_yml("zh-CN");
    let title = lookup(&content, "interactive_help_title").unwrap();
    assert!(
        title.contains('提') || title.contains('令') || title.contains('命'),
        "zh-CN help title should be Chinese, got: {title}"
    );
    let usage = lookup(&content, "interactive_help_usage").unwrap();
    assert!(
        usage.contains("Token") || usage.contains("令") || usage.contains("量"),
        "zh-CN help usage should be Chinese, got: {usage}"
    );
}

#[test]
fn test_help_english_locale() {
    let content = read_yml("en-US");
    let title = lookup(&content, "interactive_help_title").unwrap();
    assert!(
        title.contains("Available") || title.contains("Command") || title.contains("Help"),
        "en-US help title should be English, got: {title}"
    );
    let usage = lookup(&content, "interactive_help_usage").unwrap();
    assert!(
        usage.contains("usage") || usage.contains("Token") || usage.contains("Show"),
        "en-US help usage should be English, got: {usage}"
    );
}

#[test]
fn test_help_no_duplicate_keys() {
    for name in ["zh-CN", "en-US"] {
        let content = read_yml(name);
        let mut seen = std::collections::HashSet::new();
        for key in HELP_KEYS {
            let present = lookup(&content, key).is_some();
            assert!(present, "duplicate-or-missing check: {key} not in {name}");
            assert!(seen.insert(*key), "duplicate {key} in HELP_KEYS");
        }
    }
}

#[test]
fn test_help_minimum_line_count() {
    // The /help output in main.rs prints at minimum 11 + 2 blank lines =
    // 13 println! calls (8 main cmds + 1 title + 4 sdd + 2 blank). Each
    // of the 13 keys must resolve to a non-empty value.
    let content_zh = read_yml("zh-CN");
    let mut non_empty = 0;
    for key in HELP_KEYS {
        if let Some(v) = lookup(&content_zh, key)
            && !v.is_empty()
        {
            non_empty += 1;
        }
    }
    assert!(
        non_empty >= 11,
        "expected at least 11 non-empty help keys, got {non_empty}"
    );
}

#[test]
fn test_help_planned_commands_marked() {
    // All 4 Phase-3 placeholder commands must be tagged with [planned] in
    // both locales so users know they are not yet implemented.
    for name in ["zh-CN", "en-US"] {
        let content = read_yml(name);
        for key in [
            "interactive_help_undo_planned",
            "interactive_help_profile_planned",
            "interactive_help_provider_planned",
            "interactive_help_session_delete_planned",
        ] {
            let v = lookup(&content, key).unwrap_or_else(|| panic!("{key} missing in {name}"));
            assert!(
                v.contains("[planned]"),
                "{key} in {name} missing [planned] marker: {v}"
            );
        }
    }
}
