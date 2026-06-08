mod execute;
mod shell;
mod target;

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
