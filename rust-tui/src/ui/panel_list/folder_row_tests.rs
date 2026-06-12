use super::{count_style, folder_label_style};
use crate::theme::Theme;
use ratatui::style::Modifier;

#[test]
fn folder_label_uses_readable_text_without_dim() {
    let theme = Theme::default();
    let style = folder_label_style(false, false, &theme, theme.bg);

    assert_eq!(style.fg, Some(theme.fg));
    assert!(!style.add_modifier.contains(Modifier::DIM));
    assert!(style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn folder_count_uses_accent_without_dim() {
    let theme = Theme::default();
    let style = count_style(false, false, &theme, theme.bg);

    assert_eq!(style.fg, Some(theme.accent));
    assert!(!style.add_modifier.contains(Modifier::DIM));
}
