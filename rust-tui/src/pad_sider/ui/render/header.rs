use super::super::super::app::{App, NavMode};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub(super) fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode = match app.nav_mode {
        NavMode::Tree => "tree",
        NavMode::IndexMap => "index map",
        NavMode::CodexRuns => "codex runs",
    };
    let lines = vec![
        Line::from(format!("cwd: {}", app.cwd.display())),
        Line::from(format!(
            "selected: {} | lines={} bytes={} modified={}",
            app.selected_label,
            app.selected_stats.lines,
            app.selected_stats.bytes,
            app.selected_stats.modified
        )),
        Line::from(format!(
            "mode: {mode} · zoom={} · numbers={}",
            app.text_zoom,
            if app.show_line_numbers { "on" } else { "off" }
        )),
        Line::from(
            "keys: ? help · t tree · c codex runs · II index · n numbers · [/] width · / search",
        ),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" info ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
