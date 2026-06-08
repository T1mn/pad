use super::{first_command_token, safe_filename};

#[test]
fn opencode_command_uses_first_configured_token() {
    assert_eq!(
        first_command_token("/opt/bin/opencode --pure"),
        "/opt/bin/opencode"
    );
}

#[test]
fn safe_filename_sanitizes_and_falls_back() {
    assert_eq!(safe_filename("ses/../abc def"), "ses_abc_def");
    assert_eq!(safe_filename("***"), "session");
}
