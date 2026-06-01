use super::{normalize_pr_number, pr_command};
use std::ffi::OsString;

#[test]
fn pr_number_accepts_plain_hash_and_github_url() {
    assert_eq!(normalize_pr_number("123").unwrap(), "123");
    assert_eq!(normalize_pr_number("#456").unwrap(), "456");
    assert_eq!(
        normalize_pr_number("https://github.com/acme/repo/pull/789/files").unwrap(),
        "789"
    );
}

#[test]
fn pr_number_rejects_empty_zero_multiline_and_non_pr_url() {
    assert!(normalize_pr_number(" ").is_err());
    assert!(normalize_pr_number("0").is_err());
    assert!(normalize_pr_number("1\n2").is_err());
    assert!(normalize_pr_number("https://github.com/acme/repo/issues/12").is_err());
    assert!(normalize_pr_number("abc123").is_err());
}

#[test]
fn pr_command_quotes_configured_command() {
    assert_eq!(
        pr_command("123", &OsString::from("/opt/open code/bin/opencode")),
        "'/opt/open code/bin/opencode' pr 123"
    );
}
