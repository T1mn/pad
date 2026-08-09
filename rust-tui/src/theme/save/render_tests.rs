use super::*;

fn round_trip(value: &str) -> String {
    let mut content = String::new();
    push_str_line(&mut content, "api_key", value);
    let table: std::collections::HashMap<String, toml::Value> = toml::from_str(&content)
        .unwrap_or_else(|err| panic!("render {value:?} -> {content:?}: {err}"));
    table
        .get("api_key")
        .and_then(|entry| entry.as_str())
        .unwrap_or_else(|| panic!("api_key missing in {content:?}"))
        .to_string()
}

pub(crate) fn windows_style_backslash_value_round_trips() {
    assert_eq!(round_trip(r"C:\path\to\key"), r"C:\path\to\key");
}

pub(crate) fn value_ending_with_backslash_round_trips() {
    // 旧实现里结尾反斜杠会转义掉闭合引号，把整个文件结构吃掉。
    assert_eq!(round_trip(r"secret\"), r"secret\");
}

pub(crate) fn multiline_value_round_trips() {
    assert_eq!(round_trip("line1\nline2"), "line1\nline2");
    assert_eq!(round_trip("trailing\n"), "trailing\n");
    assert_eq!(round_trip("\nleading"), "\nleading");
}

pub(crate) fn quotes_tabs_and_control_characters_round_trip() {
    assert_eq!(round_trip("say \"hi\""), "say \"hi\"");
    assert_eq!(round_trip("a\tb\r\nc"), "a\tb\r\nc");
    assert_eq!(
        round_trip("bell\u{7}null-ish\u{1}"),
        "bell\u{7}null-ish\u{1}"
    );
    assert_eq!(round_trip("mixed \\ \" '"), "mixed \\ \" '");
    assert_eq!(round_trip("quote'''triple"), "quote'''triple");
}

pub(crate) fn plain_values_stay_basic_strings() {
    let mut content = String::new();
    push_str_line(&mut content, "theme", "default");
    assert_eq!(content, "theme = \"default\"\n");
}

pub(crate) fn rendered_default_config_is_parseable() {
    let rendered = render(&Config::default());
    let parsed: Result<toml::Value, _> = toml::from_str(&rendered);
    assert!(parsed.is_ok(), "default config must render valid TOML");
}
