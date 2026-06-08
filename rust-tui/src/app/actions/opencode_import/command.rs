use std::ffi::OsString;
use std::io;
use std::process::Command;

pub(super) fn import_opencode_session(source: &str, command: &OsString) -> io::Result<String> {
    let output = Command::new(command).args(["import", source]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(io::Error::other(if stderr.is_empty() {
            format!("opencode import exited with {}", output.status)
        } else {
            stderr
        }));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if stdout.is_empty() {
        source.to_string()
    } else {
        stdout
    })
}
