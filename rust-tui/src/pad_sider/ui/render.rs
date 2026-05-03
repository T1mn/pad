use super::super::app::{App, Focus, NavMode};
use super::super::preview::PreviewKind;
use super::markdown::render_markdown;
use super::overlay;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &App) {
    if let Some(preview) = app.preview.as_ref() {
        overlay::draw_preview(frame, app, preview);
        return;
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(frame.area());
    let nav_weight = match app.nav_mode {
        NavMode::Tree => app.layout_weights.tree,
        NavMode::IndexMap => app.layout_weights.index_map,
    };
    let left_total = (nav_weight + app.layout_weights.changes) as u32;
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Ratio(nav_weight as u32, left_total),
            Constraint::Ratio(app.layout_weights.changes as u32, left_total),
        ])
        .split(columns[0]);

    draw_header(frame, app, left[0]);
    draw_nav(frame, app, left[1]);
    draw_changes(frame, app, left[2]);
    draw_file_preview(frame, app, columns[1]);

    if let Some(search) = app.search.as_ref() {
        overlay::draw_search(frame, search);
    }
    if app.show_help {
        overlay::draw_help(frame);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode = match app.nav_mode {
        NavMode::Tree => "tree",
        NavMode::IndexMap => "index map",
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
        Line::from(format!("mode: {mode} · II switch tree/index map")),
        Line::from("keys: ? help · [/] width · +/- height · / search · Space full preview"),
    ];
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" info ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_nav(frame: &mut Frame, app: &App, area: Rect) {
    match app.nav_mode {
        NavMode::Tree => draw_tree(frame, app, area),
        NavMode::IndexMap => draw_index_map(frame, app, area),
    }
}

fn draw_index_map(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .index_rows
        .iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            ListItem::new(Line::from(format!("{indent}◈ {}/index.md", row.dir_label)))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.index_selected));
    let title = format!(" index map ({}) ", app.index_rows.len());
    let list = List::new(items)
        .block(focus_block(&title, app.focus == Focus::IndexMap))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
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

fn draw_file_preview(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" preview {} ", app.file_preview.title);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let paragraph = match app.file_preview.kind {
        PreviewKind::Markdown => Paragraph::new(render_markdown(&app.file_preview.content)),
        _ => Paragraph::new(Text::from(
            app.file_preview
                .content
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect::<Vec<_>>(),
        )),
    }
    .block(block)
    .wrap(Wrap { trim: false })
    .scroll((app.file_preview.scroll, 0));
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
