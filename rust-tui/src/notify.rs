use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRequest {
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NotificationBackend {
    MacOsScript,
    LinuxNotifySend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NotificationEnv<'a> {
    os: &'a str,
    has_display: bool,
    has_wayland: bool,
    has_dbus_session: bool,
}

pub fn notify_completion(request: &NotificationRequest) -> io::Result<bool> {
    let env = NotificationEnv::from_current();
    let Some(spec) = command_spec(&env, request, command_exists) else {
        return Ok(false);
    };
    spawn_notification(spec)?;
    Ok(true)
}

impl<'a> NotificationEnv<'a> {
    fn from_current() -> Self {
        Self {
            os: std::env::consts::OS,
            has_display: std::env::var_os("DISPLAY").is_some(),
            has_wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
            has_dbus_session: std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some(),
        }
    }
}

fn command_spec(
    env: &NotificationEnv<'_>,
    request: &NotificationRequest,
    has_command: impl Fn(&str) -> bool,
) -> Option<CommandSpec> {
    let backend = select_backend(env, has_command)?;
    Some(match backend {
        NotificationBackend::MacOsScript => CommandSpec {
            program: "osascript".into(),
            args: vec![
                "-e".into(),
                format!(
                    "display notification \"{}\" with title \"{}\"",
                    escape_applescript_string(&request.body),
                    escape_applescript_string(&request.title)
                ),
            ],
        },
        NotificationBackend::LinuxNotifySend => CommandSpec {
            program: "notify-send".into(),
            args: vec![
                "--app-name".into(),
                "PAD".into(),
                "--icon".into(),
                "dialog-information".into(),
                request.title.clone(),
                request.body.clone(),
            ],
        },
    })
}

fn select_backend(
    env: &NotificationEnv<'_>,
    has_command: impl Fn(&str) -> bool,
) -> Option<NotificationBackend> {
    match env.os {
        "macos" if has_command("osascript") => Some(NotificationBackend::MacOsScript),
        "linux"
            if (env.has_display || env.has_wayland || env.has_dbus_session)
                && has_command("notify-send") =>
        {
            Some(NotificationBackend::LinuxNotifySend)
        }
        _ => None,
    }
}

fn escape_applescript_string(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn spawn_notification(spec: CommandSpec) -> io::Result<()> {
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

fn command_exists(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&paths).any(|dir| executable_exists(&dir.join(program)))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> NotificationRequest {
        NotificationRequest {
            title: "PAD · Codex complete".into(),
            body: "Summarized thread title".into(),
        }
    }

    #[test]
    fn macos_uses_osascript_when_available() {
        let env = NotificationEnv {
            os: "macos",
            has_display: false,
            has_wayland: false,
            has_dbus_session: false,
        };

        let spec = command_spec(&env, &request(), |cmd| cmd == "osascript").unwrap();

        assert_eq!(spec.program, "osascript");
        assert_eq!(spec.args[0], "-e");
        assert!(spec.args[1].contains("display notification"));
    }

    #[test]
    fn macos_skips_when_osascript_missing() {
        let env = NotificationEnv {
            os: "macos",
            has_display: false,
            has_wayland: false,
            has_dbus_session: false,
        };

        assert!(command_spec(&env, &request(), |_| false).is_none());
    }

    #[test]
    fn linux_uses_notify_send_on_x11() {
        let env = NotificationEnv {
            os: "linux",
            has_display: true,
            has_wayland: false,
            has_dbus_session: false,
        };

        let spec = command_spec(&env, &request(), |cmd| cmd == "notify-send").unwrap();

        assert_eq!(spec.program, "notify-send");
        assert_eq!(spec.args[0], "--app-name");
        assert_eq!(spec.args[1], "PAD");
    }

    #[test]
    fn linux_uses_notify_send_on_wayland() {
        let env = NotificationEnv {
            os: "linux",
            has_display: false,
            has_wayland: true,
            has_dbus_session: false,
        };

        assert!(command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
    }

    #[test]
    fn linux_uses_notify_send_with_dbus_session_only() {
        let env = NotificationEnv {
            os: "linux",
            has_display: false,
            has_wayland: false,
            has_dbus_session: true,
        };

        assert!(command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
    }

    #[test]
    fn linux_skips_without_desktop_session() {
        let env = NotificationEnv {
            os: "linux",
            has_display: false,
            has_wayland: false,
            has_dbus_session: false,
        };

        assert!(command_spec(&env, &request(), |cmd| cmd == "notify-send").is_none());
    }

    #[test]
    fn linux_skips_when_notify_send_missing() {
        let env = NotificationEnv {
            os: "linux",
            has_display: true,
            has_wayland: false,
            has_dbus_session: false,
        };

        assert!(command_spec(&env, &request(), |_| false).is_none());
    }

    #[test]
    fn applescript_escapes_quotes_and_backslashes() {
        let escaped = escape_applescript_string("a\"b\\c\nd");
        assert_eq!(escaped, "a\\\"b\\\\c d");
    }

    #[test]
    fn command_exists_detects_program_in_path() {
        let temp = std::env::temp_dir().join(format!("pad-notify-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let program = temp.join("pad-test-binary");
        std::fs::write(&program, "echo test").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&program).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&program, permissions).unwrap();
        }

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", temp.as_os_str());
        assert!(command_exists("pad-test-binary"));
        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
        let _ = std::fs::remove_dir_all(&temp);
    }
}
