use super::{normalize_import_source, trim_wrapping_quotes};

#[test]
fn import_source_accepts_opencode_share_url() {
    assert_eq!(
        normalize_import_source(" https://opencode.ai/s/abc123 \n").unwrap(),
        "https://opencode.ai/s/abc123"
    );
}

#[test]
fn import_source_accepts_json_path_and_strips_quotes() {
    assert_eq!(
        normalize_import_source("'/tmp/session.sanitized.json'").unwrap(),
        "/tmp/session.sanitized.json"
    );
    assert_eq!(trim_wrapping_quotes("\"/tmp/a.json\""), "/tmp/a.json");
}

#[test]
fn import_source_rejects_multi_line_clipboard() {
    assert!(normalize_import_source("/tmp/a.json\n/tmp/b.json").is_err());
}
