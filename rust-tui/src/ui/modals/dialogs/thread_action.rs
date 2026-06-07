use super::super::common::render_modal_surface;
use super::thread_meta::draw_thread_meta_editor;
use super::thread_text::{
    thread_action_cancel_hint, thread_action_confirm_hint, thread_action_live_warning,
    thread_action_modal_confirm, thread_action_modal_title, thread_action_subject,
};
use crate::app::{App, ThreadActionKind};
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_thread_action_confirm(f: &mut Frame, app: &App, area: Rect) {
    if app.sidebar.thread_meta_editing {
        draw_thread_meta_editor(f, app, area);
        return;
    }

    let Some(action) = app.sidebar.pending_thread_action.as_ref() else {
        return;
    };

    let theme = &app.theme;
    let title = thread_action_modal_title(app.locale, action.kind);
    let subject = thread_action_subject(
        action.thread.title.as_str(),
        action.thread.session_id.as_deref(),
    );
    let confirm_line =
        thread_action_modal_confirm(app.locale, action.kind, action.thread.agent_type.clone());
    let warning =
        if action.kind == ThreadActionKind::Archive && action.thread.live_pane_id.is_some() {
            Some(thread_action_live_warning(
                app.locale,
                action.thread.agent_type.clone(),
            ))
        } else {
            None
        };

    let popup_width = 62;
    let popup_height = if warning.is_some() { 11 } else { 9 };
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

    let mut lines = vec![
        confirm_line.to_string(),
        String::new(),
        subject,
        String::new(),
    ];
    if let Some(warning) = warning {
        lines.push(warning.to_string());
        lines.push(String::new());
    }
    lines.push(thread_action_confirm_hint(app.locale).to_string());
    lines.push(thread_action_cancel_hint(app.locale).to_string());

    let block = Block::default()
        .title(format!(" ⚠ {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.warning));

    let paragraph = Paragraph::new(lines.join("\n"))
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}
