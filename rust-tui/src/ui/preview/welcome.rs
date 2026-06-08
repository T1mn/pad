use crate::i18n::{self, Locale};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_welcome_preview(
    f: &mut Frame,
    area: Rect,
    block: Block,
    locale: Locale,
    theme: &Theme,
) {
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
            i18n::t(locale, "preview.welcome"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            i18n::t(locale, "preview.subtitle"),
            Style::default().fg(theme.comment),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("─── {} ───", i18n::t(locale, "preview.keybindings")),
            Style::default().fg(theme.border),
        )),
        Line::from(""),
        keybinding_line(
            " J/K ",
            i18n::t(locale, "preview.nav_panels"),
            theme.fg,
            theme,
        ),
        keybinding_line(
            " Enter ",
            i18n::t(locale, "preview.attach"),
            theme.fg,
            theme,
        ),
        keybinding_line(" / ", i18n::t(locale, "preview.search"), theme.fg, theme),
        keybinding_line(" E ", i18n::t(locale, "preview.tree"), theme.fg, theme),
        keybinding_line(" C ", i18n::t(locale, "preview.create"), theme.fg, theme),
        keybinding_line(" D ", i18n::t(locale, "preview.delete"), theme.fg, theme),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ? ", Style::default().fg(theme.keyword)),
            Span::styled(
                i18n::t(locale, "preview.help"),
                Style::default().fg(theme.comment),
            ),
            Span::styled("  S ", Style::default().fg(theme.keyword)),
            Span::styled(
                i18n::t(locale, "preview.settings"),
                Style::default().fg(theme.comment),
            ),
            Span::styled("  Q ", Style::default().fg(theme.keyword)),
            Span::styled(
                i18n::t(locale, "preview.quit"),
                Style::default().fg(theme.comment),
            ),
        ]),
    ];
    let paragraph = Paragraph::new(welcome)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn keybinding_line<'a>(
    key: &'static str,
    label: &'a str,
    label_color: ratatui::style::Color,
    theme: &Theme,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(key, Style::default().fg(theme.keyword)),
        Span::styled(label, Style::default().fg(label_color)),
    ])
}
