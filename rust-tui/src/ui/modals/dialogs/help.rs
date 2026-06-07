use super::super::common::render_modal_surface;
use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let help_area = crate::ui::layout::popup_area(68, 32, area);

    render_modal_surface(f, help_area, theme);

    let block = Block::default()
        .title(format!(" ? {} ", crate::i18n::t(l, "help.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let paragraph = Paragraph::new(help_lines(app))
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, help_area);
}

fn help_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let l = app.locale;
    vec![
        Line::from(Span::styled(
            crate::i18n::t(l, "app.title_full"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        section_title(crate::i18n::t(l, "help.nav"), theme.warning),
        Line::from(crate::i18n::t(l, "help.move_down")),
        Line::from(crate::i18n::t(l, "help.move_up")),
        Line::from(crate::i18n::t(l, "help.jump")),
        Line::from(crate::i18n::t(l, "help.search_panels")),
        Line::from(""),
        section_title(crate::i18n::t(l, "help.actions"), theme.warning),
        Line::from(crate::i18n::t(l, "help.attach")),
        Line::from(crate::i18n::t(l, "help.create")),
        Line::from(crate::i18n::t(l, "help.delete")),
        Line::from(crate::i18n::t(l, "help.refresh")),
        Line::from(crate::i18n::t(l, "help.toggle_display_scope")),
        Line::from(crate::i18n::t(l, "help.focus_preview")),
        Line::from(crate::i18n::t(l, "help.select_preview")),
        Line::from(crate::i18n::t(l, "help.expand_preview")),
        Line::from(crate::i18n::t(l, "help.preview_back")),
        Line::from(crate::i18n::t(l, "help.scroll_preview")),
        Line::from(crate::i18n::t(l, "help.preview_home_end")),
        Line::from(""),
        section_title(crate::i18n::t(l, "help.file_tree"), theme.warning),
        Line::from(crate::i18n::t(l, "help.toggle_tree")),
        Line::from(crate::i18n::t(l, "help.tree_home")),
        Line::from(crate::i18n::t(l, "help.expand")),
        Line::from(crate::i18n::t(l, "help.go_up")),
        Line::from(crate::i18n::t(l, "help.scroll_file")),
        Line::from(crate::i18n::t(l, "help.scroll_file_page")),
        Line::from(""),
        section_title(crate::i18n::t(l, "help.other"), theme.warning),
        Line::from(crate::i18n::t(l, "help.f1")),
        Line::from(crate::i18n::t(l, "help.toggle_help")),
        Line::from(crate::i18n::t(l, "help.quit")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.detach"),
            Style::default().fg(theme.comment),
        )),
    ]
}

fn section_title(text: &'static str, color: ratatui::style::Color) -> Line<'static> {
    Line::from(Span::styled(
        text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}
