use super::thread_row::format_jump_badge;

#[test]
fn jump_badge_is_fixed_width_and_limited_to_nine() {
    assert_eq!(format_jump_badge(Some(1), 4), "#1  ");
    assert_eq!(format_jump_badge(Some(9), 4), "#9  ");
    assert_eq!(format_jump_badge(Some(10), 4), "    ");
    assert_eq!(format_jump_badge(None, 4), "    ");
}
