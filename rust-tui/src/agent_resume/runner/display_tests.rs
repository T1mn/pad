use super::display_tmux_command;

#[test]
fn display_tmux_command_quotes_args_without_collecting_segments() {
    let args = vec![
        "new-session".to_string(),
        "-s".to_string(),
        "agent session".to_string(),
        "echo ready".to_string(),
    ];

    assert_eq!(
        display_tmux_command(&args),
        "tmux new-session -s 'agent session' 'echo ready'"
    );
}
