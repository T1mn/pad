use super::parse_session_panes_output;

#[test]
fn parse_session_panes_output_extracts_pane_pid_and_command() {
    let panes = parse_session_panes_output("%1|1234|pad\n%2||zsh\n");
    assert_eq!(panes.len(), 2);
    assert_eq!(panes[0].pane_id, "%1");
    assert_eq!(panes[0].pid, Some(1234));
    assert_eq!(panes[0].command, "pad");
    assert_eq!(panes[1].pane_id, "%2");
    assert_eq!(panes[1].pid, None);
    assert_eq!(panes[1].command, "zsh");
}
