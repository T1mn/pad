use super::toast::summarize_copy_preview;

#[test]
fn copy_preview_summary_truncates_with_ascii_ellipsis() {
    assert_eq!(
        summarize_copy_preview("hello brave new world", 5),
        "hello..."
    );
}

#[test]
fn copy_preview_summary_collapses_whitespace() {
    assert_eq!(
        summarize_copy_preview(" hello\tbrave\nnew   world ", 24),
        "hello brave new world"
    );
}
