use crate::app::App;
use crate::tree::AgentLauncher;
use super::layout::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let items = app.filtered_settings_items();
    let search_h = if app.settings_searching || !app.settings_search.is_empty() { 1u16 } else { 0 };

    // Content-fit sizing: calculate minimal width/height
    let max_name = items.iter().map(|(_, _, k, _, _)| crate::i18n::t(l, k).len()).max().unwrap_or(12) as u16;
    let max_value = items.iter().map(|(_, v, _, _, _)| v.len()).max().unwrap_or(8) as u16;
    let content_w = (max_name + 2 + max_value + 4 + 2).max(22);
    let content_h = (items.len() as u16 + 2 + search_h).max(6);
    let width = content_w.min(f.area().width.saturating_sub(4));
    let height = content_h.min(f.area().height.saturating_sub(4));
    let x = (f.area().width.saturating_sub(width)) / 2;
    let y = (f.area().height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    f.render_widget(Clear, area);

    let title = format!(" ⚙ {} ", crate::i18n::t(l, "settings.title"));
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let table_area = Rect {
        x: inner.x,
        y: inner.y + search_h,
        width: inner.width,
        height: inner.height.saturating_sub(search_h),
    };
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (_id, value, name_key, _desc_key, editable))| {
            let is_selected = idx == app.settings_selected;
            let display_name = crate::i18n::t(l, name_key);

            let name_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let value_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(if *editable { theme.accent } else { theme.comment })
            } else {
                Style::default().fg(if *editable { theme.accent } else { theme.comment })
            };

            let editable_marker = if *editable { " ›" } else { "" };

            Row::new(vec![
                Cell::from(display_name).style(name_style),
                Cell::from(format!("{}{}", value, editable_marker)).style(value_style),
            ])
        })
        .collect();

    let table = Table::new(rows, [
            Constraint::Length(max_name + 1),
            Constraint::Min(0),
        ]);

    if app.settings_searching || !app.settings_search.is_empty() {
        let search_text = if app.settings_searching {
            format!("/{}|", app.settings_search)
        } else {
            format!("/{}", app.settings_search)
        };
        let search = Paragraph::new(search_text)
            .style(Style::default().fg(theme.accent));
        let search_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
        f.render_widget(search, search_area);
    }

    f.render_widget(table, table_area);
}

pub fn draw_theme_selector(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let area = centered_rect(40, 80, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" 🎨 {} [{}] ", crate::i18n::t(l, "settings.theme"), theme.name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.keyword));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

    let themes = App::available_themes();
    let rows: Vec<Row> = themes
        .iter()
        .enumerate()
        .map(|(idx, (name, desc))| {
            let is_selected = idx == app.theme_selected;
            let is_current = *name == app.config.theme;

            let marker = if is_current { "✓ " } else { "  " };

            let name_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.keyword)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(theme.keyword)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, name)).style(name_style),
                Cell::from(*desc).style(Style::default().fg(theme.comment)),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(0)])
        .header(
            Row::new(vec![
                crate::i18n::t(l, "theme.header_theme"),
                crate::i18n::t(l, "theme.header_desc"),
            ])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        );

    f.render_widget(table, inner);
}

pub fn draw_agent_launcher(f: &mut Frame, app: &App, launcher: &AgentLauncher, area: Rect) {
    let l = app.locale;
    let popup_width = 50;
    let popup_height = 12;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(area.x + popup_x, area.y + popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let items: Vec<Row> = launcher
        .agents
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let prefix = if i == launcher.selected {
                "❯ "
            } else {
                "  "
            };
            let cells = vec![Cell::from(format!("{}{}", prefix, name))];
            let style = if i == launcher.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(cells).style(style)
        })
        .collect();

    let title = format!(" {} {} ",
        crate::i18n::t(l, "agent_launcher.title"),
        launcher.target_dir.display()
    );
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let table = Table::new(items, [Constraint::Percentage(100)])
        .block(block);

    f.render_widget(table, popup_area);
}

pub fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let popup_width = 50;
    let popup_height = 8;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(area.x + popup_x, area.y + popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let panel_info = if let Some(ref panel) = app.delete_target {
        format!("{}:{}.{}", panel.session, panel.window_index, panel.pane)
    } else {
        String::from("Unknown")
    };

    let text = format!(
        "{}\n\n{}\n\n{}\n{}",
        crate::i18n::t(l, "delete.confirm_msg"),
        panel_info,
        crate::i18n::t(l, "delete.yes_hint"),
        crate::i18n::t(l, "delete.cancel_hint")
    );

    let block = Block::default()
        .title(format!(" \u{26a0}\u{fe0f} {} ", crate::i18n::t(l, "delete.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.error));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}

pub fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let popup_area = centered_rect(60, 70, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" ? {} ", crate::i18n::t(l, "help.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));

    let help_lines = vec![
        Line::from(Span::styled(
            crate::i18n::t(l, "app.title_full"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.nav"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.move_down")),
        Line::from(crate::i18n::t(l, "help.move_up")),
        Line::from(crate::i18n::t(l, "help.jump")),
        Line::from(crate::i18n::t(l, "help.search_panels")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.actions"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.attach")),
        Line::from(crate::i18n::t(l, "help.create")),
        Line::from(crate::i18n::t(l, "help.delete")),
        Line::from(crate::i18n::t(l, "help.refresh")),
        Line::from(crate::i18n::t(l, "help.scroll_preview")),
        Line::from(crate::i18n::t(l, "help.preview_home_end")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.file_tree"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.toggle_tree")),
        Line::from(crate::i18n::t(l, "help.tree_home")),
        Line::from(crate::i18n::t(l, "help.expand")),
        Line::from(crate::i18n::t(l, "help.go_up")),
        Line::from(crate::i18n::t(l, "help.scroll_file")),
        Line::from(crate::i18n::t(l, "help.scroll_file_page")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.other"),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(crate::i18n::t(l, "help.f1")),
        Line::from(crate::i18n::t(l, "help.toggle_help")),
        Line::from(crate::i18n::t(l, "help.quit")),
        Line::from(""),
        Line::from(Span::styled(
            crate::i18n::t(l, "help.detach"),
            Style::default().fg(theme.comment),
        )),
    ];

    let paragraph = Paragraph::new(help_lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

pub fn draw_relay_settings(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let area = centered_rect(55, 50, f.area());

    f.render_widget(Clear, area);

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let title = match app.relay_view {
        crate::app::state::RelayView::AgentList => format!(" {} ", crate::i18n::t(l, "relay.title")),
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => format!(
            " {} [{}] ",
            crate::i18n::t(l, "relay.providers_label"),
            selected_agent.map(|a| a.name.as_str()).unwrap_or("?")
        ),
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    };

    match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            let rows: Vec<Row> = app
                .config
                .agents
                .iter()
                .enumerate()
                .map(|(idx, agent)| {
                    let is_selected = idx == app.relay_selected_agent;
                    let active_label = agent
                        .active()
                        .map(|p| format!(" [{}]", p.label))
                        .unwrap_or_else(|| " [none]".to_string());
                    let style = if is_selected {
                        Style::default()
                            .bg(theme.highlight_bg)
                            .fg(theme.highlight_fg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.fg)
                    };
                    Row::new(vec![Cell::from(format!("{}{}", agent.name, active_label)).style(style)])
                })
                .collect();
            let table = Table::new(rows, [Constraint::Min(0)]);
            f.render_widget(table, inner);
        }
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => {
            let is_detail_focus = app.relay_view == crate::app::state::RelayView::DetailPane;
            let provider_w = inner.width * 2 / 5;
            let providers_area = Rect {
                x: inner.x,
                y: inner.y,
                width: provider_w,
                height: inner.height,
            };
            let detail_area = Rect {
                x: inner.x + provider_w + 1,
                y: inner.y,
                width: inner.width.saturating_sub(provider_w + 1),
                height: inner.height,
            };

            let prov_rows: Vec<Row> = if let Some(agent) = selected_agent {
                if agent.providers.is_empty() {
                    vec![Row::new(vec![Cell::from(crate::i18n::t(l, "relay.empty"))]).style(Style::default().fg(theme.comment))]
                } else {
                    agent.providers.iter().enumerate().map(|(i, prov)| {
                        let is_active = agent.active_provider == Some(i);
                        let selected = i == app.relay_selected_provider;
                        let active_marker = if is_active { "✓" } else { " " };
                        let test_icon = match prov.test_status {
                            Some(true) => "●",
                            Some(false) => "✗",
                            None => " ",
                        };
                        let style = if selected && !is_detail_focus {
                            Style::default()
                                .bg(theme.highlight_bg)
                                .fg(if is_active { theme.success } else { theme.highlight_fg })
                                .add_modifier(Modifier::BOLD)
                        } else if is_active {
                            Style::default().fg(theme.success).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.fg)
                        };
                        let test_color = match prov.test_status {
                            Some(true) => theme.success,
                            Some(false) => theme.error,
                            None => theme.comment,
                        };
                        Row::new(vec![Cell::from(Line::from(vec![
                            Span::styled(format!("{} ", active_marker), style),
                            Span::styled(format!("{} ", test_icon), Style::default().fg(test_color)),
                            Span::styled(prov.label.clone(), style),
                        ]))])
                    }).collect()
                }
            } else {
                vec![]
            };
            let prov_border_color = if is_detail_focus { theme.border } else { theme.accent };
            let prov_table = Table::new(prov_rows, [Constraint::Min(0)])
                .block(Block::default()
                    .title(format!(" {} ", crate::i18n::t(l, "relay.providers_label")))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(prov_border_color)));
            f.render_widget(prov_table, providers_area);

            let detail_lines: Vec<Line> = if let Some(agent) = selected_agent {
                if let Some(prov) = agent.providers.get(app.relay_selected_provider) {
                    let field = app.relay_edit_field;
                    let editing = app.relay_editing;
                    let make_val = |idx: usize, val: &str| -> String {
                        if editing && field == idx {
                            format!("{}|", app.relay_edit_buffer)
                        } else if val.is_empty() {
                            "-".to_string()
                        } else {
                            val.to_string()
                        }
                    };
                    let field_style = |idx: usize| -> Style {
                        if is_detail_focus && field == idx {
                            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                        } else if field == idx && editing {
                            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.fg)
                        }
                    };
                    let key_display = if editing && field == 2 {
                        format!("{}|", app.relay_edit_buffer)
                    } else if prov.api_key.is_empty() {
                        "-".to_string()
                    } else if prov.api_key.len() > 12 {
                        format!("{}...", &prov.api_key[..12])
                    } else {
                        prov.api_key.clone()
                    };
                    let mut lines = vec![
                        Line::from(Span::styled(crate::i18n::t(l, "relay.label"), Style::default().fg(theme.comment))),
                        Line::from(Span::styled(make_val(0, &prov.label), field_style(0))),
                        Line::from(""),
                        Line::from(Span::styled(crate::i18n::t(l, "relay.base_url"), Style::default().fg(theme.comment))),
                        Line::from(Span::styled(make_val(1, &prov.base_url), field_style(1))),
                        Line::from(""),
                        Line::from(Span::styled(crate::i18n::t(l, "relay.api_key"), Style::default().fg(theme.comment))),
                        Line::from(Span::styled(key_display, field_style(2))),
                    ];
                    // Show test result if available
                    if app.provider_test_in_progress
                        && app.relay_selected_agent == app.relay_selected_agent
                    {
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            crate::i18n::t(l, "relay.testing"),
                            Style::default().fg(theme.warning),
                        )));
                    } else if let Some(ref result) = prov.test_result {
                        lines.push(Line::from(""));
                        let color = if prov.test_status == Some(true) {
                            theme.success
                        } else {
                            theme.error
                        };
                        for line in result.lines().take(6) {
                            lines.push(Line::from(Span::styled(
                                line.to_string(),
                                Style::default().fg(color),
                            )));
                        }
                    }
                    lines
                } else {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(crate::i18n::t(l, "relay.no_provider"), Style::default().fg(theme.comment))),
                        Line::from(""),
                        Line::from(Span::styled(crate::i18n::t(l, "relay.add_hint"), Style::default().fg(theme.comment))),
                    ]
                }
            } else {
                vec![]
            };
            let detail_border_color = if is_detail_focus { theme.accent } else { theme.border };
            let detail_block = Block::default()
                .title(format!(" {} ", crate::i18n::t(l, "relay.details")))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(detail_border_color));
            let detail_para = Paragraph::new(detail_lines).block(detail_block).wrap(Wrap { trim: false });
            f.render_widget(detail_para, detail_area);
        }
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        match app.relay_view {
            crate::app::state::RelayView::AgentList => crate::i18n::t(l, "relay.footer_agent"),
            crate::app::state::RelayView::ProviderList => crate::i18n::t(l, "relay.footer_provider"),
            crate::app::state::RelayView::DetailPane => crate::i18n::t(l, "relay.footer_detail"),
        }
    };
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.comment));
    let footer_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    f.render_widget(footer, footer_area);
}
