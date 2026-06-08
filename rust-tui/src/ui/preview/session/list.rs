mod card;
mod layout;

use super::super::session_list_cache::{
    ensure_session_list_cache, selected_session_list_range, visible_session_list_lines,
};
use super::scroll::resolve_session_list_scroll;
use crate::app::App;
use crate::theme::Theme;
use ratatui::{layout::Rect, style::Style, widgets::Paragraph, Frame};

pub(crate) use card::{render_session_card, render_session_gap_line};
#[cfg(test)]
pub(crate) use layout::build_session_list_lines;
pub(crate) use layout::{session_list_total_lines, session_turn_index_at_line};

pub(super) fn draw_session_list(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    let width = area.width.max(8);
    ensure_session_list_cache(app, width, theme);

    let total_lines = session_list_total_lines(app.preview.turns.len());
    let selected_range =
        selected_session_list_range(app.preview.selected_turn, app.preview.turns.len());
    let scroll = resolve_session_list_scroll(app, selected_range, area.height, total_lines);
    let lines = visible_session_list_lines(app, width as usize, theme, scroll, area.height);
    let paragraph =
        Paragraph::new(ratatui::text::Text::from(lines)).style(Style::default().fg(theme.fg));
    f.render_widget(paragraph, area);
}
