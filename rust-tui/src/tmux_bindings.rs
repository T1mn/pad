use std::process::Command;

pub(crate) const PAD_RETURN_BINDING_MARKER: &str = "PAD_RETURN_BINDING=1;";
pub(crate) const PAD_SIDER_TOGGLE_KEYS: &[&str] = &["F10", "C-Tab"];

pub(crate) fn current_root_binding(key: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["list-keys", "-T", "root"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| line.contains(&format!(" {} ", key)))
        .filter(|line| !is_pad_managed_binding(line))
        .map(|line| line.to_string())
}

fn is_pad_managed_binding(line: &str) -> bool {
    line.contains(PAD_RETURN_BINDING_MARKER)
        || (line.contains("run-shell")
            && line.contains("tmux select-window -t '")
            && line.contains("tmux select-pane -t '")
            && (line.contains("tmux switch-client -t '")
                || line.contains("[return] before_return_select")))
}

pub(crate) fn restore_binding_cmd(saved_binding: Option<&str>, key: &str) -> String {
    saved_binding
        .map(|line| format!("tmux {}", line))
        .unwrap_or_else(|| format!("tmux unbind-key -T root {}", key))
}

pub(crate) fn pad_sider_toggle_command() -> String {
    let path = std::env::current_exe().unwrap_or_else(|_| "pad".into());
    format!(
        "{} __internal pad-sider toggle --target-pane '#{{pane_id}}'",
        shell_single_quote(&path.to_string_lossy())
    )
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
