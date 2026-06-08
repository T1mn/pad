use super::{default_width, nearest_width_level, next_width_level};

#[test]
fn default_width_is_half() {
    assert_eq!(default_width(), "50%");
}

#[test]
fn width_levels_step_without_wrapping() {
    assert_eq!(next_width_level(50, true), 55);
    assert_eq!(next_width_level(65, true), 65);
    assert_eq!(next_width_level(50, false), 45);
    assert_eq!(next_width_level(45, false), 45);
}

#[test]
fn nearest_width_level_handles_tmux_rounding() {
    assert_eq!(nearest_width_level(49), 50);
    assert_eq!(nearest_width_level(51), 50);
    assert_eq!(nearest_width_level(64), 65);
}
