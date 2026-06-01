use super::github_install_command;
use std::ffi::OsString;

#[test]
fn github_install_command_quotes_configured_command() {
    assert_eq!(
        github_install_command(&OsString::from("/opt/open code/bin/opencode")),
        "'/opt/open code/bin/opencode' github install"
    );
}
