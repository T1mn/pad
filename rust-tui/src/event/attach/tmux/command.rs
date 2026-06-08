use crate::log_debug;

pub(in crate::event::attach) fn summarize_log_text(text: &str) -> String {
    let single_line = text.trim().replace('\n', "\\n").replace('\r', "\\r");
    if single_line.is_empty() {
        return "-".to_string();
    }

    let mut shortened: String = single_line.chars().take(160).collect();
    if single_line.chars().count() > 160 {
        shortened.push('…');
    }
    shortened
}

pub(in crate::event::attach) fn run_tmux_logged(
    context: &str,
    args: Vec<String>,
) -> Option<std::process::Output> {
    log_debug!("tmux:{}: cmd=tmux {}", context, args.join(" "));

    let output = std::process::Command::new("tmux")
        .args(args.iter().map(String::as_str))
        .output()
        .ok()?;

    log_debug!(
        "tmux:{}: exit={} stdout={} stderr={}",
        context,
        output.status,
        summarize_log_text(&String::from_utf8_lossy(&output.stdout)),
        summarize_log_text(&String::from_utf8_lossy(&output.stderr))
    );

    Some(output)
}

pub(in crate::event::attach) fn run_tmux_success(context: &str, args: Vec<String>) -> bool {
    run_tmux_logged(context, args)
        .map(|output| output.status.success())
        .unwrap_or(false)
}
