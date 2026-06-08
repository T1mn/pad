use std::error::Error;
use std::process::Command;

pub const PAD_BOOTSTRAP_ENV: &str = "PAD_TMUX_BOOTSTRAPPED";
const PAD_DEFAULT_SESSION: &str = "pad";

pub fn should_bootstrap_into_tmux(
    args: &[String],
    tmux_env_present: bool,
    tmux_pane_present: bool,
    already_bootstrapped: bool,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
) -> bool {
    if crate::cli::is_info_only_command(args) || crate::cli::is_telegram_daemon_command(args) {
        return false;
    }
    if crate::cli::is_internal_command(args) {
        return false;
    }
    if already_bootstrapped {
        return false;
    }
    if tmux_env_present && tmux_pane_present {
        return false;
    }
    stdin_is_tty && stdout_is_tty
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

fn bootstrap_command(args: &[String], executable: &std::path::Path) -> String {
    let mut command = format!(
        "env {PAD_BOOTSTRAP_ENV}=1 {}",
        shell_single_quote(&executable.to_string_lossy())
    );
    for arg in args.iter().skip(1) {
        command.push(' ');
        command.push_str(&shell_single_quote(arg));
    }
    command
}

pub fn bootstrap_into_tmux(args: &[String]) -> Result<(), Box<dyn Error>> {
    let executable = std::env::current_exe()?;
    let inner_command = bootstrap_command(args, &executable);
    let mut command = Command::new("tmux");
    command.args([
        "new-session",
        "-A",
        "-s",
        PAD_DEFAULT_SESSION,
        &inner_command,
    ]);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        let err = command.exec();
        Err(Box::new(err))
    }

    #[cfg(not(unix))]
    {
        let status = command.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
