use super::{truncate_chars, truncate_for_log};

#[test]
fn truncate_for_log_keeps_existing_marker_behavior() {
    assert_eq!(truncate_for_log("abcdef", 3), "abc...");
    assert_eq!(truncate_for_log("abc", 3), "abc");
    assert_eq!(truncate_for_log("abc", 0), "...");
}

#[test]
fn truncate_chars_keeps_ellipsis_behavior() {
    assert_eq!(truncate_chars("abcdef", 4), "abc…");
    assert_eq!(truncate_chars("abc", 3), "abc");
    assert_eq!(truncate_chars("一二三四五", 4), "一二三…");
    assert_eq!(truncate_chars("abc", 0), "…");
}
