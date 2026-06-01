use super::web_command;
use std::ffi::OsString;

#[test]
fn web_command_quotes_configured_opencode_command() {
    assert_eq!(
        web_command(&OsString::from("/opt/open code/bin/opencode")),
        "'/opt/open code/bin/opencode' web"
    );
}
