use super::{browser_open_command, validate_browser_url};

#[test]
fn validates_safe_browser_urls() {
    assert!(validate_browser_url("http://localhost:3000"));
    assert!(validate_browser_url("https://example.com"));
    assert!(validate_browser_url("file:///tmp/report.html"));
    assert!(!validate_browser_url("javascript:alert(1)"));
}

#[test]
fn rejects_option_shaped_and_control_char_urls() {
    for url in [
        "-a",
        "--args",
        " -e /etc/passwd",
        "http://example.com\nhttps://evil.test",
        "http://example.com\0",
        "http://example.com\u{1b}[2J",
    ] {
        assert!(!validate_browser_url(url), "url {url:?} must be rejected");
    }
}

#[test]
fn open_command_uses_the_trimmed_url_it_validated() {
    let command = browser_open_command("  https://example.com  ").expect("trimmed url is valid");
    assert_eq!(command.args, vec!["https://example.com".to_string()]);
}

#[test]
fn open_command_strips_trailing_newline_instead_of_passing_it_on() {
    // 收尾的 CRLF 由 trim 吃掉,所以子进程拿到的和校验过的是同一个串。
    let command = browser_open_command("https://example.com\r\n").expect("trailing crlf is valid");
    assert_eq!(command.args, vec!["https://example.com".to_string()]);
}
