use crate::app::App;
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn draw_file_tree(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(ref mut tree) = app.sidebar.file_tree {
        let theme = &app.theme;
        tree.render(f, area, theme);
    } else {
        let l = app.locale;
        let block = Block::default()
            .title(format!(" {} ", crate::i18n::t(l, "tree.explorer")))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
            .border_style(Style::default().fg(app.theme.border));
        let paragraph = Paragraph::new(crate::i18n::t(l, "tree.no_dir"))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }
}
