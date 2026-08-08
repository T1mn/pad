use super::{PadRestartPlan, PadRestartTarget, PAD_CARGO_MANIFEST_DIR};

pub(super) fn execute_pad_restart_plan(plan: &PadRestartPlan) -> Result<(), String> {
    crate::log_debug!(
        "telegram: spawning native PAD restart start_dir={} command={}",
        plan.start_dir,
        plan.shell_command
    );
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    std::process::Command::new(shell)
        .args(["-lc", &plan.shell_command])
        .current_dir(&plan.start_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub(crate) fn build_pad_restart_shell_command(
    current_exe: &std::path::Path,
    current_args: &[String],
    cargo_target_dir: Option<&str>,
) -> String {
    let mut command = String::new();
    if let Some(target_dir) = cargo_target_dir.filter(|value| !value.trim().is_empty()) {
        command.push_str("export CARGO_TARGET_DIR=");
        command.push_str(&crate::shell_quote::single_quote(target_dir));
        command.push_str(" && ");
    }
    let profile = if current_exe
        .components()
        .any(|component| component.as_os_str() == "release")
    {
        "cargo build --release"
    } else {
        "cargo build"
    };
    command.push_str(profile);
    command.push_str(" && exec ");
    command.push_str(&crate::shell_quote::single_quote(
        &current_exe.to_string_lossy(),
    ));
    for argument in current_args
        .iter()
        .skip(1)
        .filter(|argument| argument.as_str() != "telegram-bot")
    {
        command.push(' ');
        command.push_str(&crate::shell_quote::single_quote(argument));
    }
    command
}

pub(super) fn current_pad_restart_plan() -> Result<PadRestartPlan, String> {
    let build_dir = std::path::Path::new(PAD_CARGO_MANIFEST_DIR);
    if !build_dir.join("Cargo.toml").exists() {
        return Err(format!(
            "cargo manifest not found in {}",
            build_dir.display()
        ));
    }
    let current_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let current_args = std::env::args().collect::<Vec<_>>();
    Ok(PadRestartPlan {
        target: PadRestartTarget::NativeProcess,
        start_dir: build_dir.to_string_lossy().to_string(),
        shell_command: build_pad_restart_shell_command(
            &current_exe,
            &current_args,
            std::env::var("CARGO_TARGET_DIR").ok().as_deref(),
        ),
    })
}
