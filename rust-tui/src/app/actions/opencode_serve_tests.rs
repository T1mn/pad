use super::serve_command;
use std::ffi::OsString;

#[test]
fn serve_command_stays_local_and_uses_random_port() {
    assert_eq!(
        serve_command(&OsString::from("/opt/open code/bin/opencode")),
        "'/opt/open code/bin/opencode' serve --hostname 127.0.0.1 --port 0"
    );
}
