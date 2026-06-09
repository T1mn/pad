use super::format_tmux_args;

#[test]
fn tmux_args_format_keeps_sider_error_shape() {
    assert_eq!(
        format_tmux_args(&["display-message", "-p", "#{window_width}"]),
        "display-message -p #{window_width}"
    );
}
