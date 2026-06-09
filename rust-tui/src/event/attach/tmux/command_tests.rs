use super::format_tmux_args;

#[test]
fn tmux_args_format_keeps_attach_log_shape() {
    let args = vec![
        "display-message".to_string(),
        "-p".to_string(),
        "#{pane_id}".to_string(),
    ];

    assert_eq!(format_tmux_args(&args), "display-message -p #{pane_id}");
}
