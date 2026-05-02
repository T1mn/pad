use super::super::app::{App, Focus};
use super::overlay;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(preview) = app.preview.as_ref() {
        overlay::draw_preview(frame, app, preview);
        return;
    }

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(frame.area());

    draw_header(frame, app, areas[0]);
    draw_tree(frame, app, areas[1]);
    draw_changes(frame, app, areas[2]);

    if let Some(search) = app.search.as_ref() {
        overlay::draw_search(frame, search);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(format!("cwd: {}", app.cwd.display())),
        Line::from(format!(
            "target: {}",
            app.target_pane.as_deref().unwrap_or("-")
        )),
        Line::from(format!(
            "selected: {} | lines={} bytes={} modified={}",
            app.selected_label,
            app.selected_stats.lines,
            app.selected_stats.bytes,
            app.selected_stats.modified
        )),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" info ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_tree(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .tree
        .iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            let marker = if row.is_dir {
                if row.expanded {
                    "▾"
                } else {
                    "▸"
                }
            } else {
                "•"
            };
            ListItem::new(Line::from(format!("{}{} {}", indent, marker, row.label)))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.selected));
    let list = List::new(items)
        .block(focus_block(" tree ", app.focus == Focus::Tree))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_changes(frame: &mut Frame, app: &App, area: Rect) {
    let text = Text::from(
        app.changes
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect::<Vec<_>>(),
    );
    let paragraph = Paragraph::new(text)
        .block(focus_block(" changes ", app.focus == Focus::Changes))
        .wrap(Wrap { trim: false })
        .scroll((app.changes_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn focus_block(title: &str, focused: bool) -> Block<'static> {
    let mut block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().add_modifier(Modifier::BOLD));
    }
    block
}
