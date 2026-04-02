use std::io;
#[cfg(any(target_os = "linux", test))]
use std::path::Path;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRequest {
    pub title: String,
    pub body: String,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NotificationEnv {
    has_display: bool,
    has_wayland: bool,
    has_dbus_session: bool,
}

pub fn notify(request: &NotificationRequest) -> io::Result<bool> {
    if notifications_disabled() {
        let _ = request;
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let spec = macos::command_spec(request);
        spawn_notification(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        let env = NotificationEnv::from_current();
        let Some(spec) = linux_command_spec(&env, request, command_exists) else {
            return Ok(false);
        };
        spawn_notification(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = request;
        Ok(false)
    }
}

pub fn notify_completion(request: &NotificationRequest) -> io::Result<bool> {
    notify(request)
}

fn notifications_disabled() -> bool {
    cfg!(test) || std::env::var_os("PAD_DISABLE_NOTIFICATIONS").is_some()
}

#[cfg(target_os = "linux")]
impl NotificationEnv {
    fn from_current() -> Self {
        Self {
            has_display: std::env::var_os("DISPLAY").is_some(),
            has_wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
            has_dbus_session: std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some(),
        }
    }
}

#[cfg(any(target_os = "linux", test))]
fn linux_command_spec(
    env: &NotificationEnv,
    request: &NotificationRequest,
    has_command: impl Fn(&str) -> bool,
) -> Option<LinuxCommandSpec> {
    if (env.has_display || env.has_wayland || env.has_dbus_session) && has_command("notify-send") {
        Some(LinuxCommandSpec {
            program: "notify-send".into(),
            args: vec![
                "--app-name".into(),
                "PAD".into(),
                "--icon".into(),
                "dialog-information".into(),
                request.title.clone(),
                request.body.clone(),
            ],
        })
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct LinuxCommandSpec {
    program: String,
    args: Vec<String>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn spawn_notification(program: &str, args: &[String]) -> io::Result<()> {
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
fn command_exists(program: &str) -> bool {
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

#[cfg(any(target_os = "macos", test))]
mod macos {
    use super::NotificationRequest;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(super) struct MacCommandSpec {
        pub program: String,
        pub args: Vec<String>,
    }

    pub(super) fn command_spec(request: &NotificationRequest) -> MacCommandSpec {
        MacCommandSpec {
            program: "osascript".into(),
            args: vec![
                "-e".into(),
                "on run argv".into(),
                "-e".into(),
                "display notification (item 2 of argv) with title (item 1 of argv)".into(),
                "-e".into(),
                "end run".into(),
                request.title.clone(),
                request.body.clone(),
            ],
        }
    }
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
    fn linux_uses_notify_send_on_x11() {
        let env = NotificationEnv {
            has_display: true,
            has_wayland: false,
            has_dbus_session: false,
        };

        let spec = linux_command_spec(&env, &request(), |cmd| cmd == "notify-send").unwrap();

        assert_eq!(spec.program, "notify-send");
        assert_eq!(spec.args[0], "--app-name");
        assert_eq!(spec.args[1], "PAD");
    }

    #[test]
    fn linux_uses_notify_send_on_wayland() {
        let env = NotificationEnv {
            has_display: false,
            has_wayland: true,
            has_dbus_session: false,
        };

        assert!(linux_command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
    }

    #[test]
    fn linux_uses_notify_send_with_dbus_session_only() {
        let env = NotificationEnv {
            has_display: false,
            has_wayland: false,
            has_dbus_session: true,
        };

        assert!(linux_command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
    }

    #[test]
    fn linux_skips_without_desktop_session() {
        let env = NotificationEnv {
            has_display: false,
            has_wayland: false,
            has_dbus_session: false,
        };

        assert!(linux_command_spec(&env, &request(), |cmd| cmd == "notify-send").is_none());
    }

    #[test]
    fn linux_skips_when_notify_send_missing() {
        let env = NotificationEnv {
            has_display: true,
            has_wayland: false,
            has_dbus_session: false,
        };

        assert!(linux_command_spec(&env, &request(), |_| false).is_none());
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

    #[test]
    fn macos_uses_osascript_with_argument_passing() {
        let spec = macos::command_spec(&request());

        assert_eq!(spec.program, "osascript");
        assert_eq!(spec.args[0], "-e");
        assert_eq!(spec.args[1], "on run argv");
        assert_eq!(
            spec.args[3],
            "display notification (item 2 of argv) with title (item 1 of argv)"
        );
        assert_eq!(spec.args[6], request().title);
        assert_eq!(spec.args[7], request().body);
    }
}
