use super::labels::{
    special_view_empty_back_hint, special_view_empty_hint, special_view_empty_title,
};
use crate::app::state::ThreadListView;
use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub(super) fn empty_message(
    locale: Locale,
    view: ThreadListView,
    theme: &Theme,
) -> Vec<Line<'static>> {
    if view != ThreadListView::Normal {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                special_view_empty_title(locale, view),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                special_view_empty_hint(locale, view),
                Style::default().fg(theme.fg),
            )),
            Line::from(Span::styled(
                special_view_empty_back_hint(locale, view),
                Style::default().fg(theme.comment),
            )),
        ];
    }

    vec![
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(locale, "panel.native_title"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(locale, "panel.native_hint"),
            Style::default().fg(theme.fg),
        )),
        Line::from(Span::styled(
            crate::i18n::t(locale, "panel.native_focus"),
            Style::default().fg(theme.comment),
        )),
    ]
}
