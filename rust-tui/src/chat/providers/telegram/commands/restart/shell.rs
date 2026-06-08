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

    steps.push(build_command(current_exe));
    steps.push(exec_command(current_exe, current_args));

    steps.join(" && ")
}

fn build_command(current_exe: &std::path::Path) -> String {
    if restart_uses_release_profile(current_exe) {
        "cargo build --release".to_string()
    } else {
        "cargo build".to_string()
    }
}

fn exec_command(current_exe: &std::path::Path, current_args: &[String]) -> String {
    let mut parts = vec![
        "exec".to_string(),
        shell_single_quote(&current_exe.to_string_lossy()),
    ];
    for arg in pad_restart_args(current_args) {
        parts.push(shell_single_quote(&arg));
    }
    parts.join(" ")
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
