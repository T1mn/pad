mod animation;
mod folder_row;
mod metrics;
mod style;
mod thread_row;

use crate::app::state::FocusTarget;
use crate::app::App;
use crate::i18n::Locale;
use crate::sidebar::SidebarItem;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, Wrap,
    },
    Frame,
};

pub fn draw_panel_list(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme.clone();
    let l = app.locale;
    let archived_threads_view = app.sidebar.archived_threads_view;
    let showing_live = app.showing_live_sessions();
    let panel_is_focused = !app.sidebar.show_tree && app.preview.focus == FocusTarget::Panel;
    let selected_idx = app.table_state.selected();
    let expanded_folders = app.sidebar.expanded_folders.clone();
    let hovered_folder_key = app.sidebar.hovered_folder_key.clone();

    let item_count = visible_thread_count(app.visible_sidebar_items_ref());
    let border_color = if panel_is_focused {
        theme.border_focused
    } else {
        theme.border
    };
    let focus_mark = if panel_is_focused { "●" } else { "○" };
    let title = if archived_threads_view {
        format!(
            " {} {} {} ",
            focus_mark,
            archived_title_label(l),
            item_count
        )
    } else {
        format!(
            " {} {} {} ",
            focus_mark,
            display_scope_title_label(l, showing_live),
            item_count
        )
    };
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (show_scrollbar, actual_item_count) = {
        let items = app.visible_sidebar_items_ref();
        let actual_item_count = items.len();
        let content_width = inner.width as usize;
        let is_minimal = content_width < 12;
        let total_h: usize = items
            .iter()
            .map(|item| match item {
                SidebarItem::Folder(_) => 1,
                SidebarItem::Thread(_) => {
                    if is_minimal {
                        1
                    } else {
                        2
                    }
                }
            })
            .sum();
        let show_scrollbar = total_h > inner.height as usize;

        if items.is_empty() {
            let empty_msg = if archived_threads_view {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        archived_empty_title(l),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        archived_empty_hint(l),
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        archived_empty_back_hint(l),
                        Style::default().fg(theme.comment),
                    )),
                ]
            } else {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        crate::i18n::t(l, "panel.empty_title"),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        crate::i18n::t(l, "panel.empty_hint"),
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        crate::i18n::t(l, "panel.empty_agents"),
                        Style::default().fg(theme.accent),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        crate::i18n::t(l, "panel.empty_create"),
                        Style::default().fg(theme.fg),
                    )),
                    Line::from(Span::styled(
                        crate::i18n::t(l, "panel.empty_refresh"),
                        Style::default().fg(theme.comment),
                    )),
                ]
            };
            let empty = Paragraph::new(empty_msg)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            f.render_widget(empty, inner);
        } else {
            let rows: Vec<Row> = items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    build_sidebar_row(
                        item,
                        idx == selected_idx.unwrap_or(usize::MAX),
                        content_width,
                        &theme,
                        expanded_folders.contains(item.folder_key()),
                        hovered_folder_key.as_deref() == Some(item.folder_key()),
                    )
                })
                .collect();

            let table = Table::new(rows, [Constraint::Min(0)])
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Never);

            let mut table_state = app.table_state;
            f.render_stateful_widget(table, inner, &mut table_state);
            app.table_state = table_state;
        }
        (show_scrollbar, actual_item_count)
    };

    if show_scrollbar && actual_item_count > 0 {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(actual_item_count).position(selected_idx.unwrap_or(0));
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn build_sidebar_row(
    item: &SidebarItem,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    is_expanded: bool,
    is_hovered_folder: bool,
) -> Row<'static> {
    match item {
        SidebarItem::Folder(folder) => folder_row::build_folder_row(
            folder,
            is_selected,
            content_width,
            theme,
            is_expanded,
            is_hovered_folder,
        ),
        SidebarItem::Thread(thread) => {
            thread_row::build_thread_row(thread, is_selected, content_width, theme)
        }
    }
}

fn archived_title_label(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "归档"
    } else {
        "Archived"
    }
}

fn display_scope_title_label(locale: Locale, live_only: bool) -> &'static str {
    if is_cjk_locale(locale) {
        if live_only {
            "在线"
        } else {
            "全部"
        }
    } else if live_only {
        "LIVE"
    } else {
        "ALL"
    }
}

fn archived_empty_title(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "没有归档会话"
    } else {
        "No archived threads"
    }
}

fn archived_empty_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "当前没有可恢复的归档会话"
    } else {
        "There are no archived threads to restore"
    }
}

fn archived_empty_back_hint(locale: Locale) -> &'static str {
    if is_cjk_locale(locale) {
        "按 'Z' 返回普通视图"
    } else {
        "Press 'Z' to return to the main view"
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}

fn visible_thread_count(items: &[SidebarItem]) -> usize {
    items
        .iter()
        .filter(|item| matches!(item, SidebarItem::Thread(_)))
        .count()
}

pub fn preferred_panel_width(app: &mut App) -> u16 {
    let archived_threads_view = app.sidebar.archived_threads_view;
    let locale = app.locale;
    let live_only = app.showing_live_sessions();
    let title_width = if archived_threads_view {
        metrics::display_width(&format!(
            " {} {} {} ",
            "○",
            archived_title_label(locale),
            88
        ))
    } else {
        metrics::display_width(&format!(
            " {} {} {} ",
            "○",
            display_scope_title_label(locale, live_only),
            888
        ))
    };
    let items = app.visible_sidebar_items_ref();
    let content_width = items
        .iter()
        .map(|item| match item {
            SidebarItem::Folder(folder) => {
                2 + metrics::display_width(&metrics::truncate_to_width(&folder.label, 28))
            }
            SidebarItem::Thread(thread) => {
                let subtitle = thread_row::thread_subtitle(thread);
                let title_width =
                    metrics::display_width(&metrics::truncate_to_width(&thread.title, 34));
                let subtitle_width =
                    metrics::display_width(&metrics::truncate_to_width(&subtitle, 32));
                9 + title_width.max(subtitle_width)
            }
        })
        .max()
        .unwrap_or(10);
    (content_width.max(title_width) as u16 + 4).clamp(6, 42)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AgentPanel;

    #[test]
    fn shimmer_preserves_text_content() {
        let text = "rust-tui";
        let rendered: String = animation::shimmer_spans(
            text,
            ratatui::style::Color::White,
            ratatui::style::Color::Cyan,
            ratatui::style::Color::Black,
        )
        .into_iter()
        .map(|span| span.content.to_string())
        .collect();
        assert_eq!(rendered, text);
    }

    #[test]
    fn preferred_panel_width_keeps_short_name_visible() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "kanban".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: crate::model::AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: true,
            state: crate::model::AgentState::Busy,
            state_source: crate::model::AgentStateSource::Scanner,
            transcript_path: None,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            git_info: None,
            pid: None,
            start_time: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        assert!(preferred_panel_width(&mut app) >= 13);
    }

    #[test]
    fn waiting_threads_do_not_breathe() {
        assert!(animation::thread_badge_breathes(
            &crate::model::AgentState::Busy
        ));
        assert!(!animation::thread_badge_breathes(
            &crate::model::AgentState::Waiting
        ));
        assert!(!animation::thread_badge_breathes(
            &crate::model::AgentState::Idle
        ));
    }

    #[test]
    fn visible_thread_count_ignores_folder_rows() {
        let folder = crate::sidebar::SidebarFolder {
            key: "folder:/tmp".into(),
            path: "/tmp".into(),
            label: "tmp".into(),
            updated_at: 0,
            threads: Vec::new(),
        };
        let thread = crate::sidebar::SidebarThread {
            key: "thread:1".into(),
            folder_key: folder.key.clone(),
            working_dir: "/tmp".into(),
            folder_label: "tmp".into(),
            agent_type: crate::model::AgentType::Codex,
            runtime_source: None,
            session_id: Some("session-1".into()),
            transcript_path: None,
            title: "Test".into(),
            upstream_title: None,
            subtitle: None,
            title_override: None,
            note: None,
            tags: Vec::new(),
            pinned: false,
            updated_at: 0,
            sort_updated_at: 0,
            live_pane_id: None,
            live_location: None,
            pid: None,
            git_info: None,
            state: crate::model::AgentState::Idle,
            is_active: false,
            cached_preview_turns: Vec::new(),
            session_cache_state: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
            archived: false,
        };
        let items = vec![SidebarItem::Folder(folder), SidebarItem::Thread(thread)];

        assert_eq!(visible_thread_count(&items), 1);
    }
}
