use super::{parse_json_string, parse_version};

#[test]
fn parse_codex_version_from_cli_output() {
    assert_eq!(
        parse_version("codex-cli 0.125.0"),
        Some("0.125.0".to_string())
    );
}

#[test]
fn parse_npm_version_json_output() {
    assert_eq!(
        parse_json_string("\"0.125.0\""),
        Some("0.125.0".to_string())
    );
}
