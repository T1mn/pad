mod common;
mod file_preview;
mod layout;
mod markdown;
mod plain;
mod session;
mod session_list_cache;

use crate::app::state::FocusTarget;
use crate::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub const PREVIEW_INFO_CARD_HEIGHT: u16 = 9;
pub(crate) const DETAIL_SMOOTH_SPAN_THRESHOLD: usize = 320;
pub(crate) const DETAIL_SMOOTH_LINE_THRESHOLD: usize = 72;

pub use file_preview::draw_file_preview;
pub use layout::{extract_preview_selection_text, preview_sid_text_at};
pub use session::render_session_detail_lines;
pub(crate) use session::session_turn_index_at_line;

pub fn draw_preview(f: &mut Frame, app: &mut App, area: Rect) {
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
        let welcome = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                " PAD ",
                Style::default()
                    .bg(theme.accent)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.welcome"),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                crate::i18n::t(l, "preview.subtitle"),
                Style::default().fg(theme.comment),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("─── {} ───", crate::i18n::t(l, "preview.keybindings")),
                Style::default().fg(theme.border),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(" J/K ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.nav_panels"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Enter ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.attach"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" / ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.search"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" E ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.tree"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" C ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.create"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(" D ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.delete"),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" ? ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.help"),
                    Style::default().fg(theme.comment),
                ),
                Span::styled("  S ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.settings"),
                    Style::default().fg(theme.comment),
                ),
                Span::styled("  Q ", Style::default().fg(theme.keyword)),
                Span::styled(
                    crate::i18n::t(l, "preview.quit"),
                    Style::default().fg(theme.comment),
                ),
            ]),
        ];
        let paragraph = Paragraph::new(welcome)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
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
