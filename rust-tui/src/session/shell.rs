pub(super) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

pub(super) fn wrap_tmux_run_shell(script: &str) -> String {
    format!("sh -lc {}", shell_single_quote(script))
}

pub(super) fn shell_trace_log_cmd(trace_id: &str, stage: &str, details: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    let message = format!("[handoff trace={}] stage={} {}", trace_id, stage, details);
    format!(
        "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
        shell_single_quote(&message),
        shell_single_quote(&log_path)
    )
}
