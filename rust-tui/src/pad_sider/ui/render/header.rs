use super::super::super::app::{App, NavMode};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode = match app.nav_mode {
        NavMode::Tree => "tree",
        NavMode::IndexMap => "index map",
        NavMode::CodexRuns => "codex runs",
    };
    let cwd_line = Line::from(vec![
        Span::raw("cwd: "),
        Span::raw(app.cwd.to_string_lossy()),
    ]);
    let selected_line = Line::from(vec![
        Span::raw("selected: "),
        Span::raw(app.selected_label.as_str()),
        Span::raw(" | lines="),
        Span::raw(app.selected_stats.lines.to_string()),
        Span::raw(" bytes="),
        Span::raw(app.selected_stats.bytes.to_string()),
        Span::raw(" modified="),
        Span::raw(app.selected_stats.modified.as_str()),
    ]);
    let mode_line = Line::from(vec![
        Span::raw("mode: "),
        Span::raw(mode),
        Span::raw(" · zoom="),
        Span::raw(app.text_zoom.to_string()),
        Span::raw(" · numbers="),
        Span::raw(if app.show_line_numbers { "on" } else { "off" }),
    ]);
    let lines = vec![
        cwd_line,
        selected_line,
        mode_line,
        Line::from(
            "keys: ? help · t tree · c codex runs · II index · n numbers · [/] width · / search",
        ),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" info ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
