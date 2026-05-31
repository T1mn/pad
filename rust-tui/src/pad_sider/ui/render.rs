use super::super::app::{App, Focus, NavMode};
use super::super::preview::PreviewKind;
use super::diff::render_diff_patch;
use super::line_numbers::{add_line_numbers, text_lines};
use super::markdown::render_markdown;
use super::overlay;
use super::split;
use super::text_zoom::apply_text_zoom;
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

    let (left_area, preview_area) = split::split_columns(frame.area());
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(left_area);

    draw_header(frame, app, left[0]);
    draw_nav(frame, app, left[1]);
    draw_file_preview(frame, app, preview_area);

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

fn draw_nav(frame: &mut Frame, app: &App, area: Rect) {
    match app.nav_mode {
        NavMode::Tree => draw_tree(frame, app, area),
        NavMode::IndexMap => draw_index_map(frame, app, area),
        NavMode::CodexRuns => draw_codex_runs(frame, app, area),
    }
}

fn draw_codex_runs(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .codex_diffs
        .iter()
        .map(|entry| {
            let status = match entry.status {
                crate::codex_turn_diff::TurnDiffStatus::Running => "●",
                crate::codex_turn_diff::TurnDiffStatus::Completed => "✓",
            };
            let prompt = crate::codex_turn_diff::prompt_summary(entry.prompt.as_deref(), 44);
            let time = entry.ended_at.as_deref().unwrap_or(&entry.started_at);
            ListItem::new(Line::from(format!(
                "{status} {time}  {} files +{} -{}  {prompt}",
                entry.stats.files_changed, entry.stats.insertions, entry.stats.deletions
            )))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(app.codex_diff_selected));
    let title = format!(" codex runs ({}) ", app.codex_diffs.len());
    let list = List::new(items)
        .block(focus_block(&title, app.focus == Focus::CodexRuns))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_index_map(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .index_rows
        .iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            let label = if row.dir_label == "." {
                "index.md".to_string()
            } else {
                format!("{}/index.md", row.dir_label)
            };
            ListItem::new(Line::from(format!("{indent}◈ {label}")))
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

fn draw_file_preview(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" preview {} ", app.file_preview.title);
    let text = match app.file_preview.kind {
        PreviewKind::Markdown => render_markdown(&app.file_preview.content),
        PreviewKind::Diff => {
            render_diff_patch(&app.file_preview.content, area.width.saturating_sub(2))
        }
        _ => text_lines(&app.file_preview.content),
    };
    let text = with_preview_display_options(text, app.show_line_numbers, app.text_zoom);
    let paragraph = Paragraph::new(text)
        .block(focus_block(&title, app.focus == Focus::Preview))
        .wrap(Wrap { trim: false })
        .scroll((app.file_preview.scroll, 0));
    frame.render_widget(paragraph, area);
}

pub(super) fn with_preview_display_options(
    text: Text<'static>,
    show_line_numbers: bool,
    text_zoom: i8,
) -> Text<'static> {
    let text = if show_line_numbers {
        add_line_numbers(text)
    } else {
        text
    };
    apply_text_zoom(text, text_zoom)
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
