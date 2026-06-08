use super::*;

fn request() -> NotificationRequest {
    NotificationRequest {
        title: "PAD · Codex complete".into(),
        body: "Summarized thread title".into(),
    }
}

#[test]
fn linux_uses_notify_send_on_x11() {
    let env = linux::NotificationEnv {
        has_display: true,
        has_wayland: false,
        has_dbus_session: false,
    };

    let spec = linux::command_spec(&env, &request(), |cmd| cmd == "notify-send").unwrap();

    assert_eq!(spec.program, "notify-send");
    assert_eq!(spec.args[0], "--app-name");
    assert_eq!(spec.args[1], "PAD");
}

#[test]
fn linux_uses_notify_send_on_wayland() {
    let env = linux::NotificationEnv {
        has_display: false,
        has_wayland: true,
        has_dbus_session: false,
    };

    assert!(linux::command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
}

#[test]
fn linux_uses_notify_send_with_dbus_session_only() {
    let env = linux::NotificationEnv {
        has_display: false,
        has_wayland: false,
        has_dbus_session: true,
    };

    assert!(linux::command_spec(&env, &request(), |cmd| cmd == "notify-send").is_some());
}

#[test]
fn linux_skips_without_desktop_session() {
    let env = linux::NotificationEnv {
        has_display: false,
        has_wayland: false,
        has_dbus_session: false,
    };

    assert!(linux::command_spec(&env, &request(), |cmd| cmd == "notify-send").is_none());
}

#[test]
fn linux_skips_when_notify_send_missing() {
    let env = linux::NotificationEnv {
        has_display: true,
        has_wayland: false,
        has_dbus_session: false,
    };

    assert!(linux::command_spec(&env, &request(), |_| false).is_none());
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
    unsafe {
        std::env::set_var("PATH", temp.as_os_str());
    }
    assert!(command::command_exists("pad-test-binary"));
    unsafe {
        if let Some(path) = original_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
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
