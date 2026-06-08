use super::parse_pane_line;

#[test]
fn parse_pane_line_keeps_pipe_inside_working_dir() {
    let pane = parse_pane_line("s|w|1|0|%1|123|zsh|/tmp/a|b").unwrap();

    assert_eq!(pane.session, "s");
    assert_eq!(pane.pane_pid, "123");
    assert_eq!(pane.working_dir, "/tmp/a|b");
}
