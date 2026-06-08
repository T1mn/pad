use super::super::*;
use ratatui::style::Color;

#[test]
fn readability_boost_keeps_status_text_close_to_primary_fg() {
    let theme = Theme::by_name("catppuccin");
    assert_eq!(theme.status_fg, theme.fg);
}

#[test]
fn readability_boost_lifts_comment_contrast() {
    let boosted = Theme::by_name("one-dark");
    assert_ne!(boosted.comment, Color::Rgb(92, 99, 112));
}
