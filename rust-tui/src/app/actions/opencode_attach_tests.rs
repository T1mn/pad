use super::{attach_command, normalize_server_url};
use std::ffi::OsString;

#[test]
fn attach_url_accepts_single_http_url_and_strips_quotes() {
    assert_eq!(
        normalize_server_url("'http://localhost:4096/'").unwrap(),
        "http://localhost:4096"
    );
    assert_eq!(
        normalize_server_url("https://example.com/path").unwrap(),
        "https://example.com/path"
    );
}

#[test]
fn attach_url_rejects_multi_line_or_non_http_clipboard() {
    assert!(normalize_server_url("http://a:1\nhttp://b:2").is_err());
    assert!(normalize_server_url("ftp://example.com:21").is_err());
    assert!(normalize_server_url("https://").is_err());
    assert!(normalize_server_url("https://example .com").is_err());
}

#[test]
fn attach_command_quotes_url_and_command() {
    assert_eq!(
        attach_command(
            "http://localhost:4096/a'b",
            &OsString::from("/opt/opencode")
        ),
        "'/opt/opencode' attach 'http://localhost:4096/a'\\''b'"
    );
}
