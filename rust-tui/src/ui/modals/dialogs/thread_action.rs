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

    let body = thread_action_confirm_body(
        &confirm_line,
        &subject,
        warning,
        thread_action_confirm_hint(app.locale),
        thread_action_cancel_hint(app.locale),
    );

    let block = Block::default()
        .title(format!(" ⚠ {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.warning));

    let paragraph = Paragraph::new(body)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

fn thread_action_confirm_body(
    confirm_line: &str,
    subject: &str,
    warning: Option<&str>,
    confirm_hint: &str,
    cancel_hint: &str,
) -> String {
    let mut body = String::new();
    push_body_line(&mut body, confirm_line);
    push_body_line(&mut body, "");
    push_body_line(&mut body, subject);
    push_body_line(&mut body, "");
    if let Some(warning) = warning {
        push_body_line(&mut body, warning);
        push_body_line(&mut body, "");
    }
    push_body_line(&mut body, confirm_hint);
    push_body_line(&mut body, cancel_hint);
    body
}

fn push_body_line(body: &mut String, line: &str) {
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(line);
}

#[cfg(test)]
#[path = "thread_action_tests.rs"]
mod tests;
