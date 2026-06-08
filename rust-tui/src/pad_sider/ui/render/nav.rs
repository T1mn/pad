use super::super::super::app::{App, Focus, NavMode};
use super::super::file_icons;
use super::super::nav_window::{list_viewport_height, relative_selection, selected_window};
use super::focus_block;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
    Frame,
};

pub(super) fn draw_nav(frame: &mut Frame, app: &App, area: Rect) {
    match app.nav_mode {
        NavMode::Tree => draw_tree(frame, app, area),
        NavMode::IndexMap => draw_index_map(frame, app, area),
        NavMode::CodexRuns => draw_codex_runs(frame, app, area),
    }
}

fn draw_codex_runs(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" codex runs ({}) ", app.codex_diffs.len());
    let block = focus_block(&title, app.focus == Focus::CodexRuns);
    let range = selected_window(
        app.codex_diffs.len(),
        app.codex_diff_selected,
        list_viewport_height(area.height),
    );
    let items = app
        .codex_diffs
        .get(range.clone())
        .unwrap_or_default()
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
    state.select(relative_selection(app.codex_diff_selected, &range));
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_index_map(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" index map ({}) ", app.index_rows.len());
    let block = focus_block(&title, app.focus == Focus::IndexMap);
    let range = selected_window(
        app.index_rows.len(),
        app.index_selected,
        list_viewport_height(area.height),
    );
    let items = app
        .index_rows
        .get(range.clone())
        .unwrap_or_default()
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
    state.select(relative_selection(app.index_selected, &range));
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_tree(frame: &mut Frame, app: &App, area: Rect) {
    let block = focus_block(" tree ", app.focus == Focus::Tree);
    let range = selected_window(
        app.tree.len(),
        app.selected,
        list_viewport_height(area.height),
    );
    let items = app
        .tree
        .get(range.clone())
        .unwrap_or_default()
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
            let icon = file_icons::icon(&row.label, row.is_dir);
            let accent = file_icons::accent(&row.label, row.is_dir);
            ListItem::new(Line::from(vec![
                Span::styled(indent, Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{marker} "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{icon} "), Style::default().fg(accent)),
                Span::styled(
                    row.label.clone(),
                    Style::default().fg(Color::Rgb(212, 212, 212)),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(relative_selection(app.selected, &range));
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut state);
}
