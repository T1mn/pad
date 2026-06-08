use super::format_line;
use crate::theme::Theme;

#[test]
fn format_line_detects_error_case_insensitively() {
    let theme = Theme::default();
    let spans = format_line("FAILED to run", &theme);

    assert_eq!(spans[0].style.fg, Some(theme.error));
}

#[test]
fn format_line_detects_success_case_insensitively() {
    let theme = Theme::default();
    let spans = format_line("SUCCESS", &theme);

    assert_eq!(spans[0].style.fg, Some(theme.success));
}
