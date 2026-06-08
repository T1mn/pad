pub(super) const LIST_PANES_FORMAT: &str = "#{session_name}|#{window_name}|#{window_index}|#{pane_index}|#{pane_id}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}";

#[derive(Clone, Copy)]
pub(super) struct PaneLine<'a> {
    pub(super) session: &'a str,
    pub(super) window: &'a str,
    pub(super) window_index: &'a str,
    pub(super) pane: &'a str,
    pub(super) pane_id: &'a str,
    pub(super) pane_pid: &'a str,
    pub(super) current_cmd: &'a str,
    pub(super) working_dir: &'a str,
}

pub(super) fn parse_pane_line(line: &str) -> Option<PaneLine<'_>> {
    let mut parts = line.splitn(8, '|');
    Some(PaneLine {
        session: parts.next()?,
        window: parts.next()?,
        window_index: parts.next()?,
        pane: parts.next()?,
        pane_id: parts.next()?,
        pane_pid: parts.next()?,
        current_cmd: parts.next()?,
        working_dir: parts.next()?,
    })
}

#[cfg(test)]
#[path = "tmux_panes_tests.rs"]
mod tests;
