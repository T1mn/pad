mod animation;
mod draw;
mod empty;
mod file_tree {
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
}
mod folder_row;
mod labels;
mod metrics;
mod status {
    use crate::app::App;
    use ratatui::{
        layout::Rect,
        style::Style,
        widgets::{Block, Borders, Paragraph},
        Frame,
    };

    pub fn draw_agent_status_bar(f: &mut Frame, app: &App, area: Rect) {
        let l = app.locale;
        let active = app.panels.iter().filter(|p| p.is_active).count();
        let total = app.panels.len();
        let tmpl = crate::i18n::t(l, "panel.agent_count");
        let text = format!(
            " {} ",
            tmpl.replacen("{}", &total.to_string(), 1)
                .replacen("{}", &active.to_string(), 1)
        );
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
            .border_style(Style::default().fg(app.theme.border));
        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
    }
}
mod style;
mod thread_row;
mod viewport;
mod width;

#[cfg(test)]
mod tests;

pub use draw::draw_panel_list;
pub use file_tree::draw_file_tree;
pub use status::draw_agent_status_bar;
pub use width::preferred_panel_width;
