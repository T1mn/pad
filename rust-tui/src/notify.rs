use std::io;

#[cfg(any(target_os = "linux", target_os = "macos", test))]
mod command;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(any(target_os = "macos", test))]
mod macos;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRequest {
    pub title: String,
    pub body: String,
}

pub fn notify(request: &NotificationRequest) -> io::Result<bool> {
    if notifications_disabled() {
        let _ = request;
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        let spec = macos::command_spec(request);
        command::spawn_notification(&spec.program, &spec.args)?;
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        let env = linux::NotificationEnv::from_current();
        let Some(spec) = linux::command_spec(&env, request, command::command_exists) else {
            return Ok(false);
        };
        command::spawn_notification(&spec.program, &spec.args)?;
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
