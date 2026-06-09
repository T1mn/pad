use super::{trailing_chars, truncate_modal_line_middle};

#[test]
fn truncate_modal_line_middle_keeps_existing_shape() {
    assert_eq!(truncate_modal_line_middle("abcdefghijkl", 8), "ab...jkl");
    assert_eq!(
        truncate_modal_line_middle("一二三四五六七八", 7),
        "一二...七八"
    );
}

#[test]
fn truncate_modal_line_middle_handles_short_width() {
    assert_eq!(truncate_modal_line_middle("abcd", 3), "...");
    assert_eq!(truncate_modal_line_middle("abcde", 4), "...e");
}

#[test]
fn trailing_chars_handles_zero_count() {
    assert_eq!(trailing_chars("abcd", 0), "");
}
