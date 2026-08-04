mod common;
mod file_preview;
mod layout;
mod markdown;
mod plain;
mod session;
mod session_list_cache;
mod welcome;

use crate::app::state::FocusTarget;
use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub const PREVIEW_INFO_CARD_HEIGHT: u16 = 11;
pub(crate) const DETAIL_SMOOTH_SPAN_THRESHOLD: usize = 320;
pub(crate) const DETAIL_SMOOTH_LINE_THRESHOLD: usize = 72;

pub use file_preview::draw_file_preview;
pub use layout::{extract_preview_selection_text, preview_share_url_text_at, preview_sid_text_at};
pub use session::render_session_detail_lines;
pub(crate) use session::session_turn_index_at_line;

pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
    if app.terminal_is_active() {
        draw_native_terminal(f, app, area);
        return;
    }

    let theme = app.theme.clone();
    let l = app.locale;
    let preview_is_focused = app.preview.focus == FocusTarget::Preview;
    let focus_mark = if preview_is_focused { "●" } else { "○" };
    let title = format!(" {} {} ", focus_mark, crate::i18n::t(l, "preview.title"));

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(if preview_is_focused {
            theme.border_focused
        } else {
            theme.border
        }));

    if app.panels.is_empty() {
        welcome::draw_welcome_preview(f, area, block, l, &theme);
        return;
    }

    let selected_thread = if app.preview_navigation_debounce_active() {
        app.preview_target_thread()
            .or_else(|| app.selected_preview_thread())
    } else {
        app.selected_preview_thread()
    };
    if let Some(thread) = selected_thread {
        let inner = block.inner(area);
        f.render_widget(block.clone(), area);

        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(PREVIEW_INFO_CARD_HEIGHT),
                Constraint::Min(0),
            ])
            .split(inner);

        layout::draw_preview_info_card(f, app, split[0], &theme, &thread);

        if app.config.codex.show_qa_preview
            && app.preview.source == crate::model::PreviewSource::Session
            && !app.preview.turns.is_empty()
        {
            session::draw_session_preview(f, app, split[1], &theme);
        } else if app.preview.source == crate::model::PreviewSource::Session
            && !app.preview.turns.is_empty()
        {
            let blank = Paragraph::new("").style(Style::default().bg(theme.bg).fg(theme.comment));
            f.render_widget(blank, split[1]);
        } else {
            plain::draw_plain_preview(f, app, split[1], false, &block, &theme);
        }
    } else {
        plain::draw_plain_preview(f, app, area, true, &block, &theme);
    }
}

fn draw_native_terminal(f: &mut Frame, app: &App, area: Rect) {
    if let Some(frame) = app.terminal_frame().cloned() {
        f.render_widget(
            crate::terminal_runtime::TerminalPaneWidget::new(&frame)
                .focused(app.terminal_is_focused()),
            area,
        );
        if let Some(exit) = app.terminal_exit() {
            let status = exit
                .code
                .map(|code| format!(" exited {code} "))
                .unwrap_or_else(|| " signaled ".to_string());
            let width = status.len().min(usize::from(area.width)) as u16;
            let status_area = Rect::new(
                area.x + area.width.saturating_sub(width + 1),
                area.y + area.height.saturating_sub(1),
                width,
                1,
            );
            f.render_widget(
                Paragraph::new(status)
                    .style(Style::default().bg(app.theme.bg).fg(app.theme.warning)),
                status_area,
            );
        }
        return;
    }

    let focused = app.terminal_is_focused();
    let block = Block::default()
        .title(" Native Terminal ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            app.theme.border_focused
        } else {
            app.theme.border
        }));
    let message = app
        .terminal_error()
        .map(|error| format!("Terminal failed: {error}"))
        .unwrap_or_else(|| "Starting native shell…".to_string());
    f.render_widget(
        Paragraph::new(message)
            .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
            .block(block),
        area,
    );
}
