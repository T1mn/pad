use super::super::common::{render_modal_surface, truncate_modal_line_middle};
use super::thread_text::{
    thread_action_subject, thread_meta_editor_field_label, thread_meta_editor_help_text,
    thread_meta_editor_prompt_text, thread_meta_editor_title,
};
use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_thread_meta_editor(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let Some(thread) = app.sidebar.thread_meta_target.as_ref() else {
        return;
    };

    let title = thread_meta_editor_title(l, app.sidebar.thread_meta_edit_kind);
    let subject = thread_action_subject(thread.title.as_str(), thread.session_id.as_deref());
    let field_label = thread_meta_editor_field_label(l, app.sidebar.thread_meta_edit_kind);
    let help_text = thread_meta_editor_help_text(l);
    let prompt_text = thread_meta_editor_prompt_text(l, app.sidebar.thread_meta_edit_kind);

    let popup_width = 72;
    let popup_height = 10;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );
    let subject_width = 34.min(popup_width.saturating_sub(4) as usize);
    let value_width = popup_width.saturating_sub(10) as usize;

    render_modal_surface(f, popup_area, theme);

    let subject = truncate_modal_line_middle(&subject, subject_width);
    let paragraph = Paragraph::new(editor_lines(
        app,
        &subject,
        field_label,
        prompt_text,
        help_text,
        value_width,
    ))
    .block(editor_block(theme, title))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

fn editor_lines<'a>(
    app: &'a App,
    subject: &'a str,
    field_label: &'static str,
    prompt_text: &'static str,
    help_text: &'static str,
    value_width: usize,
) -> Vec<Line<'a>> {
    let theme = &app.theme;
    let cursor_value = format!(
        "{}|",
        truncate_modal_line_middle(
            &app.sidebar.thread_meta_buffer,
            value_width.saturating_sub(1)
        )
    );
    vec![
        Line::from(Span::styled(
            subject,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            field_label,
            Style::default().fg(theme.comment),
        )),
        Line::from(Span::styled(
            cursor_value,
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            prompt_text,
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::DIM),
        )),
        Line::from(Span::styled(help_text, Style::default().fg(theme.comment))),
    ]
}

fn editor_block(theme: &crate::theme::Theme, title: &'static str) -> Block<'static> {
    Block::default()
        .title(format!(" ✎ {} ", title))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent))
}
