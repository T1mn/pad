use super::super::super::common::render_modal_surface;
use crate::app::App;
use crate::ui::layout::popup_area;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub(super) fn draw_model_edit_popup(f: &mut Frame, app: &App, parent: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let edit_area = popup_area(44, 5, parent);
    render_modal_surface(f, edit_area, theme);
    let edit_inner = Rect {
        x: edit_area.x + 2,
        y: edit_area.y + 1,
        width: edit_area.width.saturating_sub(4),
        height: edit_area.height.saturating_sub(2),
    };
    let label = if app.relay_popup_field == 0 {
        crate::i18n::t(locale, "relay.model_id")
    } else {
        crate::i18n::t(locale, "relay.model_name")
    };
    let lines = vec![
        Line::from(Span::styled(
            label,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(
            format!("{}|", app.relay_popup_buffer),
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    f.render_widget(Paragraph::new(lines), edit_inner);
}
