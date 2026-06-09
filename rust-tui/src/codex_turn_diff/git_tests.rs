use super::format_git_args;

#[test]
fn git_args_format_keeps_error_message_shape() {
    assert_eq!(
        format_git_args(&["diff", "--no-ext-diff", "base", "end"]),
        "diff --no-ext-diff base end"
    );
}
