mod execute {
    use super::super::{PadRestartPlan, PadRestartTarget};

    pub(in crate::chat::providers::telegram::commands) fn execute_pad_restart_plan(
        plan: &PadRestartPlan,
    ) -> Result<(), String> {
        crate::log_debug!(
            "telegram: executing pad restart target={:?} start_dir={} command={}",
            plan.target,
            plan.start_dir,
            plan.shell_command
        );

        match &plan.target {
            PadRestartTarget::RespawnPane(pane_id) => crate::tmux_dispatch::respawn_pane_shell(
                pane_id,
                &plan.start_dir,
                &plan.shell_command,
            )
            .map_err(|err| err.to_string()),
            PadRestartTarget::NewDetachedSession(session_name) => {
                crate::tmux_dispatch::new_detached_session_shell(
                    session_name,
                    &plan.start_dir,
                    &plan.shell_command,
                )
                .map_err(|err| err.to_string())
            }
        }
    }
}
mod shell {
    pub(crate) fn build_pad_restart_shell_command(
        current_exe: &std::path::Path,
        current_args: &[String],
        cargo_target_dir: Option<&str>,
    ) -> String {
        let mut command = String::new();
        if let Some(cargo_target_dir) = cargo_target_dir.filter(|value| !value.trim().is_empty()) {
            command.push_str("export CARGO_TARGET_DIR=");
            command.push_str(&shell_single_quote(cargo_target_dir));
            command.push_str(" && ");
        }

        command.push_str(&build_command(current_exe));
        command.push_str(" && ");
        command.push_str(&exec_command(current_exe, current_args));
        command
    }

    fn build_command(current_exe: &std::path::Path) -> String {
        if restart_uses_release_profile(current_exe) {
            "cargo build --release".to_string()
        } else {
            "cargo build".to_string()
        }
    }

    fn exec_command(current_exe: &std::path::Path, current_args: &[String]) -> String {
        let mut command = String::from("exec ");
        command.push_str(&shell_single_quote(&current_exe.to_string_lossy()));
        for arg in pad_restart_args(current_args) {
            command.push(' ');
            command.push_str(&shell_single_quote(&arg));
        }
        command
    }

    fn restart_uses_release_profile(current_exe: &std::path::Path) -> bool {
        current_exe
            .components()
            .any(|component| component.as_os_str() == "release")
    }

    fn pad_restart_args(current_args: &[String]) -> Vec<String> {
        current_args
            .iter()
            .skip(1)
            .filter(|arg| arg.as_str() != "telegram-bot")
            .cloned()
            .collect()
    }

    fn shell_single_quote(value: &str) -> String {
        crate::shell_quote::single_quote(value)
    }
}
mod target {
    use super::super::{PadRestartTarget, PAD_DEFAULT_SESSION_NAME};

    pub(super) fn current_pad_restart_target(
        current_exe: &std::path::Path,
    ) -> Result<PadRestartTarget, String> {
        let current_tmux_pane = std::env::var("TMUX_PANE")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let pad_status_pid = crate::runtime_status::read_status(&crate::paths::pad_status_path())
            .filter(|status| crate::runtime_status::process_alive(status.pid))
            .map(|status| status.pid);

        let panes = if current_tmux_pane.is_some() {
            Vec::new()
        } else if crate::tmux_dispatch::session_exists(PAD_DEFAULT_SESSION_NAME)
            .map_err(|err| err.to_string())?
        {
            crate::tmux_dispatch::list_session_panes(PAD_DEFAULT_SESSION_NAME)
                .map_err(|err| err.to_string())?
        } else {
            Vec::new()
        };

        let expected_command = current_exe
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("pad");

        Ok(select_pad_restart_target(
            current_tmux_pane.as_deref(),
            PAD_DEFAULT_SESSION_NAME,
            &panes,
            pad_status_pid,
            expected_command,
        ))
    }

    pub(crate) fn select_pad_restart_target(
        current_tmux_pane: Option<&str>,
        session_name: &str,
        session_panes: &[crate::tmux_dispatch::SessionPaneInfo],
        pad_pid: Option<u32>,
        expected_command: &str,
    ) -> PadRestartTarget {
        if let Some(pane_id) = current_tmux_pane.filter(|value| !value.trim().is_empty()) {
            return PadRestartTarget::RespawnPane(pane_id.to_string());
        }

        if let Some(pid) = pad_pid {
            if let Some(pane) = session_panes.iter().find(|pane| pane.pid == Some(pid)) {
                return PadRestartTarget::RespawnPane(pane.pane_id.clone());
            }
        }

        if let Some(pane) = session_panes
            .iter()
            .find(|pane| pane.command.trim() == expected_command)
        {
            return PadRestartTarget::RespawnPane(pane.pane_id.clone());
        }

        if let Some(first) = session_panes.first() {
            return PadRestartTarget::RespawnPane(first.pane_id.clone());
        }

        PadRestartTarget::NewDetachedSession(session_name.to_string())
    }
}

use super::{PadRestartPlan, PAD_CARGO_MANIFEST_DIR};

pub(super) use execute::execute_pad_restart_plan;
#[cfg(test)]
pub(crate) use shell::build_pad_restart_shell_command;
#[cfg(test)]
pub(crate) use target::select_pad_restart_target;

pub(super) fn current_pad_restart_plan() -> Result<PadRestartPlan, String> {
    let build_dir = std::path::Path::new(PAD_CARGO_MANIFEST_DIR);
    if !build_dir.join("Cargo.toml").exists() {
        return Err(format!(
            "cargo manifest not found in {}",
            build_dir.display()
        ));
    }

    let current_exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let current_args = std::env::args().collect::<Vec<_>>();
    let shell_command = shell::build_pad_restart_shell_command(
        &current_exe,
        &current_args,
        std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
    );
    let target = target::current_pad_restart_target(&current_exe)?;

    Ok(PadRestartPlan {
        target,
        start_dir: build_dir.to_string_lossy().to_string(),
        shell_command,
    })
}
