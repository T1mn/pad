pub(in crate::event::attach) fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(in crate::event::attach) fn wrap_tmux_run_shell(script: &str) -> String {
    format!("sh -lc {}", shell_single_quote(script))
}

pub(in crate::event::attach) fn shell_log_cmd(message: &str) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        "printf '[%s] %s\\n' \"$(date '+%H:%M:%S')\" {} >> {}",
        shell_single_quote(&format!("[return] {}", message)),
        shell_single_quote(&log_path)
    )
}

pub(in crate::event::attach) fn wait_for_zoom_flag_cmd(
    target_pane_id: &str,
    expected_zoomed: &str,
    label: &str,
) -> String {
    let log_path = crate::paths::log_path().to_string_lossy().to_string();
    format!(
        concat!(
            "_pad_wait_i=0; _pad_zoom=''; ",
            "while [ $_pad_wait_i -lt 30 ]; do ",
            "_pad_zoom=$(tmux display-message -t {} -p '#{{window_zoomed_flag}}' 2>/dev/null | tr -d '\\r\\n'); ",
            "[ \"$_pad_zoom\" = {} ] && break; ",
            "_pad_wait_i=$((_pad_wait_i + 1)); ",
            "sleep 0.01; ",
            "done; ",
            "printf '%s\\n' \"[return] {} target_pane={} zoomed=${{_pad_zoom:-?}} tries=${{_pad_wait_i}}\" >> {}"
        ),
        shell_single_quote(target_pane_id),
        shell_single_quote(expected_zoomed),
        label,
        target_pane_id,
        shell_single_quote(&log_path)
    )
}
