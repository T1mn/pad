use super::format_tmux_args;

#[test]
fn tmux_arg_format_keeps_space_separated_debug_shape() {
    assert_eq!(
        format_tmux_args(&["send-keys", "-t", "%1", "C-m"]),
        "send-keys -t %1 C-m"
    );
}
