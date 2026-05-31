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
mod tests {
    use super::parse_pane_line;

    #[test]
    fn parse_pane_line_keeps_pipe_inside_working_dir() {
        let pane = parse_pane_line("s|w|1|0|%1|123|zsh|/tmp/a|b").unwrap();

        assert_eq!(pane.session, "s");
        assert_eq!(pane.pane_pid, "123");
        assert_eq!(pane.working_dir, "/tmp/a|b");
    }
}
