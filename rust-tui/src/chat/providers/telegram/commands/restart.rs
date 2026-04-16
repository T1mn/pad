use super::*;

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
    let shell_command = build_pad_restart_shell_command(
        &current_exe,
        &current_args,
        std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
    );
    let target = current_pad_restart_target(&current_exe)?;

    Ok(PadRestartPlan {
        target,
        start_dir: build_dir.to_string_lossy().to_string(),
        shell_command,
    })
}

fn current_pad_restart_target(current_exe: &std::path::Path) -> Result<PadRestartTarget, String> {
    let current_tmux_pane = std::env::var("TMUX_PANE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let pad_status_pid = runtime_status::read_status(&crate::paths::pad_status_path())
        .filter(|status| runtime_status::process_alive(status.pid))
        .map(|status| status.pid);

    let panes = if current_tmux_pane.is_some() {
        Vec::new()
    } else if tmux_dispatch::session_exists(PAD_DEFAULT_SESSION_NAME)
        .map_err(|err| err.to_string())?
    {
        tmux_dispatch::list_session_panes(PAD_DEFAULT_SESSION_NAME)
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

pub(crate) fn build_pad_restart_shell_command(
    current_exe: &std::path::Path,
    current_args: &[String],
    cargo_target_dir: Option<&str>,
) -> String {
    let mut steps = Vec::new();
    if let Some(cargo_target_dir) = cargo_target_dir.filter(|value| !value.trim().is_empty()) {
        steps.push(format!(
            "export CARGO_TARGET_DIR={}",
            shell_single_quote(cargo_target_dir)
        ));
    }

    let build_cmd = if restart_uses_release_profile(current_exe) {
        "cargo build --release".to_string()
    } else {
        "cargo build".to_string()
    };
    steps.push(build_cmd);

    let mut exec_parts = vec![
        "exec".to_string(),
        shell_single_quote(&current_exe.to_string_lossy()),
    ];
    for arg in pad_restart_args(current_args) {
        exec_parts.push(shell_single_quote(&arg));
    }
    steps.push(exec_parts.join(" "));

    steps.join(" && ")
}

pub(super) fn execute_pad_restart_plan(plan: &PadRestartPlan) -> Result<(), String> {
    log_debug!(
        "telegram: executing pad restart target={:?} start_dir={} command={}",
        plan.target,
        plan.start_dir,
        plan.shell_command
    );

    match &plan.target {
        PadRestartTarget::RespawnPane(pane_id) => {
            tmux_dispatch::respawn_pane_shell(pane_id, &plan.start_dir, &plan.shell_command)
                .map_err(|err| err.to_string())
        }
        PadRestartTarget::NewDetachedSession(session_name) => {
            tmux_dispatch::new_detached_session_shell(
                session_name,
                &plan.start_dir,
                &plan.shell_command,
            )
            .map_err(|err| err.to_string())
        }
    }
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
    format!("'{}'", value.replace('\'', r#"'\''"#))
}
