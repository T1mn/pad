use super::render_selection_title_line;
use crate::theme::Theme;
use crate::ui::selection::SelectionItem;
use ratatui::style::Style;

#[test]
fn title_line_marks_selection_and_right_aligns_value() {
    let theme = Theme::default();
    let item = SelectionItem {
        title: "Theme".into(),
        value: Some("dark  ›".into()),
        ..Default::default()
    };

    let line = render_selection_title_line(&item, 20, &theme, theme.bg, true, Style::default());

    assert_eq!(line.spans[0].content.as_ref(), "❯ ");
    assert_eq!(line.spans[1].content.as_ref(), "Theme");
    assert_eq!(line.spans[3].content.as_ref(), "dark  ›");
    assert_eq!(line.spans[3].style.fg, Some(theme.highlight_fg));
}

#[test]
fn unselected_value_uses_accent_color() {
    let theme = Theme::default();
    let item = SelectionItem {
        title: "Sound".into(),
        value: Some("on  ›".into()),
        ..Default::default()
    };

    let line = render_selection_title_line(&item, 20, &theme, theme.bg, false, Style::default());

    assert_eq!(line.spans[0].content.as_ref(), "  ");
    assert_eq!(line.spans[3].style.fg, Some(theme.accent));
}
