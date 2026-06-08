use super::NotificationRequest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NotificationEnv {
    pub(super) has_display: bool,
    pub(super) has_wayland: bool,
    pub(super) has_dbus_session: bool,
}

#[cfg(target_os = "linux")]
impl NotificationEnv {
    pub(super) fn from_current() -> Self {
        Self {
            has_display: std::env::var_os("DISPLAY").is_some(),
            has_wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
            has_dbus_session: std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some(),
        }
    }
}

pub(super) fn command_spec(
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LinuxCommandSpec {
    pub(super) program: String,
    pub(super) args: Vec<String>,
}
