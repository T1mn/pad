use super::format_tmux_args;

#[test]
fn tmux_args_format_keeps_error_message_shape() {
    let args = vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        "pad-resume-demo".to_string(),
    ];

    assert_eq!(format_tmux_args(&args), "new-session -d -s pad-resume-demo");
}
