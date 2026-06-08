use super::{FileTree, TreeEntry};
use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

impl FileTree {
    /// Get icon for file type
    fn file_icon(entry: &TreeEntry) -> &'static str {
        if entry.is_dir {
            if entry.name == ".." {
                "⬆️"
            } else if entry.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            let name = &entry.name;
            if name.ends_with(".rs") {
                "🦀"
            } else if name.ends_with(".py") {
                "🐍"
            } else if name.ends_with(".js") || name.ends_with(".ts") {
                "📜"
            } else if name.ends_with(".go") {
                "🔵"
            } else if name.ends_with(".java") {
                "☕"
            } else if name.ends_with(".md") {
                "📝"
            } else if name.ends_with(".json")
                || name.ends_with(".toml")
                || name.ends_with(".yaml")
                || name.ends_with(".yml")
            {
                "⚙️"
            } else if name.ends_with(".sh") || name.ends_with(".bash") || name.ends_with(".zsh") {
                "🐚"
            } else if name.ends_with(".html") || name.ends_with(".css") {
                "🌐"
            } else {
                "📄"
            }
        }
    }

    /// Render tree view
    pub fn render(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        // Create list items
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let icon = Self::file_icon(entry);
                let content = format!("{} {}", icon, entry.name);

                let style = if entry.is_dir {
                    Style::default().fg(theme.accent)
                } else {
                    Style::default().fg(theme.fg)
                };

                ListItem::new(Line::from(vec![Span::styled(content, style)]))
            })
            .collect();

        // Create block with title
        let title = format!("📁 {}", self.current_path.display());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_focused));

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        );

        f.render_stateful_widget(list, area, &mut self.state);
    }
}
