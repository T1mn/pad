use std::io;
#[cfg(any(target_os = "linux", test))]
use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn spawn_notification(program: &str, args: &[String]) -> io::Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

#[cfg(any(target_os = "linux", test))]
pub(super) fn command_exists(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&paths).any(|dir| executable_exists(&dir.join(program)))
}

#[cfg(any(target_os = "linux", test))]
fn executable_exists(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| {
            if !meta.is_file() {
                return false;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o111 != 0
            }
            #[cfg(not(unix))]
            {
                true
            }
        })
        .unwrap_or(false)
}
