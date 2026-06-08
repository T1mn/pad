#[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
use std::io;
use std::path::Path;
#[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
use std::process::{Command, Stdio};

#[cfg(any(target_os = "linux", target_os = "macos", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CommandSpec {
    pub(super) program: String,
    pub(super) args: Vec<String>,
}

#[cfg(any(target_os = "linux", test))]
pub(super) fn linux_command_spec(
    path: &Path,
    has_command: impl Fn(&str) -> bool,
) -> Option<CommandSpec> {
    let file = path.to_string_lossy().into_owned();
    if has_command("paplay") {
        return Some(CommandSpec {
            program: "paplay".into(),
            args: vec![file],
        });
    }
    if has_command("pw-play") {
        return Some(CommandSpec {
            program: "pw-play".into(),
            args: vec![file],
        });
    }
    if has_command("aplay") {
        return Some(CommandSpec {
            program: "aplay".into(),
            args: vec!["-q".into(), file],
        });
    }
    if has_command("play") {
        return Some(CommandSpec {
            program: "play".into(),
            args: vec!["-q".into(), file],
        });
    }
    None
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn macos_command_spec(
    path: &Path,
    has_command: impl Fn(&str) -> bool,
) -> Option<CommandSpec> {
    if !has_command("afplay") {
        return None;
    }
    Some(CommandSpec {
        program: "afplay".into(),
        args: vec![path.to_string_lossy().into_owned()],
    })
}

#[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
pub(super) fn spawn_audio(program: &str, args: &[String]) -> io::Result<()> {
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

#[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
pub(super) fn command_exists(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&paths).any(|dir| executable_exists(&dir.join(program)))
}

#[cfg(all(any(target_os = "linux", target_os = "macos"), not(test)))]
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
