use super::layout::popup_area;
use crate::app::App;
use crate::theme::Theme;
use crate::tree::AgentLauncher;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

fn render_modal_surface(f: &mut Frame, area: Rect, theme: &Theme) {
    f.render_widget(Clear, area);
    let surface = Block::default().style(Style::default().bg(theme.bg).fg(theme.fg));
    f.render_widget(surface, area);
}

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let items = app.filtered_settings_items();
    let search_h = if app.settings_searching || !app.settings_search.is_empty() {
        1u16
    } else {
        0
    };

    let max_name = items
        .iter()
        .map(|(_, _, k, _, _)| crate::i18n::t(l, k).len())
        .max()
        .unwrap_or(12) as u16;
    let max_value = items
        .iter()
        .map(|(_, v, _, _, _)| v.len())
        .max()
        .unwrap_or(8) as u16;
    let content_w = (max_name + max_value + 6).max(20);
    let content_h = items.len() as u16 + search_h;
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let title = format!(" ⚙ {} ", crate::i18n::t(l, "settings.title"));
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
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
                Style::default().bg(theme.highlight_bg).fg(if *editable {
                    theme.accent
                } else {
                    theme.comment
                })
            } else {
                Style::default().fg(if *editable {
                    theme.accent
                } else {
                    theme.comment
                })
            };

            let editable_marker = if *editable { " ›" } else { "" };

            Row::new(vec![
                Cell::from(display_name).style(name_style),
                Cell::from(format!("{}{}", value, editable_marker)).style(value_style),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(max_name + 1), Constraint::Min(0)]);

    if app.settings_searching || !app.settings_search.is_empty() {
        let search_text = if app.settings_searching {
            format!("/{}|", app.settings_search)
        } else {
            format!("/{}", app.settings_search)
        };
        let search = Paragraph::new(search_text).style(Style::default().fg(theme.accent));
        let search_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        };
        f.render_widget(search, search_area);
    }

    f.render_widget(table, table_area);
}

pub fn draw_theme_selector(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let themes = App::available_themes();

    let max_name_len = themes.iter().map(|(n, _)| n.len()).max().unwrap_or(10) as u16;
    let max_desc_len = themes.iter().map(|(_, d)| d.len()).max().unwrap_or(10) as u16;
    let content_w = (max_name_len + max_desc_len + 6).max(26);
    let content_h = themes.len() as u16 + 2; // items + header
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(format!(
            " 🎨 {} [{}] ",
            crate::i18n::t(l, "settings.theme"),
            theme.name
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
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

    let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(0)]).header(
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
    let theme = &app.theme;
    let l = app.locale;
    let popup_width = 50;
    let popup_height = 12;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

    let items: Vec<Row> = launcher
        .agents
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let prefix = if i == launcher.selected { "❯ " } else { "  " };
            let cells = vec![Cell::from(format!("{}{}", prefix, name))];
            let style = if i == launcher.selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            Row::new(cells).style(style)
        })
        .collect();

    let title = format!(
        " {} {} ",
        crate::i18n::t(l, "agent_launcher.title"),
        launcher.target_dir.display()
    );
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let table = Table::new(items, [Constraint::Percentage(100)]).block(block);

    f.render_widget(table, popup_area);
}

pub fn draw_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let popup_width = 50;
    let popup_height = 8;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(
        area.x + popup_x,
        area.y + popup_y,
        popup_width,
        popup_height,
    );

    render_modal_surface(f, popup_area, theme);

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
        .title(format!(
            " \u{26a0}\u{fe0f} {} ",
            crate::i18n::t(l, "delete.title")
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
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
    let help_area = popup_area(54, 29, area);

    render_modal_surface(f, help_area, theme);

    let block = Block::default()
        .title(format!(" ? {} ", crate::i18n::t(l, "help.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
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
        Line::from(crate::i18n::t(l, "help.focus_preview")),
        Line::from(crate::i18n::t(l, "help.select_preview")),
        Line::from(crate::i18n::t(l, "help.expand_preview")),
        Line::from(crate::i18n::t(l, "help.preview_back")),
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

    f.render_widget(paragraph, help_area);
}

pub fn draw_relay_settings(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let agent_count = app.config.agents.len() as u16;
    let max_prov_count = app
        .config
        .agents
        .iter()
        .map(|a| a.providers.len())
        .max()
        .unwrap_or(1) as u16;
    let max_label = app
        .config
        .agents
        .iter()
        .flat_map(|a| {
            std::iter::once(a.name.len()).chain(a.providers.iter().map(|p| p.label.len() + 4))
        })
        .max()
        .unwrap_or(10) as u16;
    let content_w = (max_label.max(20).max(30) * 3 / 2) as u16; // 1.5x width
    let content_h = (agent_count.max(max_prov_count).max(3) + 1) * 3 / 2; // 1.5x height
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let title = match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            format!(" {} ", crate::i18n::t(l, "relay.title"))
        }
        crate::app::state::RelayView::ProviderList | crate::app::state::RelayView::DetailPane => {
            format!(
                " {} [{}] ",
                crate::i18n::t(l, "relay.providers_label"),
                selected_agent.map(|a| a.name.as_str()).unwrap_or("?")
            )
        }
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
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
                    Row::new(vec![
                        Cell::from(format!("{}{}", agent.name, active_label)).style(style)
                    ])
                })
                .collect();
            let table = Table::new(rows, [Constraint::Min(0)]);
            f.render_widget(table, inner);
        }
        crate::app::state::RelayView::ProviderList => {
            let prov_rows: Vec<Row> = if let Some(agent) = selected_agent {
                if agent.providers.is_empty() {
                    vec![Row::new(vec![Cell::from(crate::i18n::t(l, "relay.empty"))])
                        .style(Style::default().fg(theme.comment))]
                } else {
                    agent
                        .providers
                        .iter()
                        .enumerate()
                        .map(|(i, prov)| {
                            let is_active = agent.active_provider == Some(i);
                            let selected = i == app.relay_selected_provider;
                            let active_marker = if is_active { "✓" } else { " " };
                            // Dot color: active=green, test_fail=dark_red, default=gray
                            let dot_color = match prov.test_status {
                                Some(false) => Color::Rgb(180, 60, 60), // dark red
                                Some(true) => theme.success,            // green
                                None => {
                                    if is_active {
                                        theme.success
                                    } else {
                                        theme.comment
                                    }
                                } // green if active, gray otherwise
                            };
                            let style = if selected {
                                Style::default()
                                    .bg(theme.highlight_bg)
                                    .fg(if is_active {
                                        theme.success
                                    } else {
                                        theme.highlight_fg
                                    })
                                    .add_modifier(Modifier::BOLD)
                            } else if is_active {
                                Style::default()
                                    .fg(theme.success)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.fg)
                            };
                            Row::new(vec![Cell::from(Line::from(vec![
                                Span::styled(format!("{} ", active_marker), style),
                                Span::styled("● ", Style::default().fg(dot_color)),
                                Span::styled(prov.label.clone(), style),
                            ]))])
                        })
                        .collect()
                }
            } else {
                vec![]
            };
            let prov_table = Table::new(prov_rows, [Constraint::Min(0)]);
            f.render_widget(prov_table, inner);
        }
        crate::app::state::RelayView::DetailPane => {
            // Render provider list underneath (dimmed)
            let prov_rows: Vec<Row> = if let Some(agent) = selected_agent {
                agent
                    .providers
                    .iter()
                    .enumerate()
                    .map(|(i, prov)| {
                        let is_active = agent.active_provider == Some(i);
                        let marker = if is_active { "✓ " } else { "  " };
                        let style = Style::default().fg(theme.comment);
                        Row::new(vec![
                            Cell::from(format!("{}{}", marker, prov.label)).style(style)
                        ])
                    })
                    .collect()
            } else {
                vec![]
            };
            let prov_table = Table::new(prov_rows, [Constraint::Min(0)]);
            f.render_widget(prov_table, inner);
        }
    }

    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        match app.relay_view {
            crate::app::state::RelayView::AgentList => crate::i18n::t(l, "relay.footer_agent"),
            crate::app::state::RelayView::ProviderList => {
                crate::i18n::t(l, "relay.footer_provider")
            }
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

pub fn draw_language_selector(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;
    let locales = App::available_locales();

    let max_name_len = locales
        .iter()
        .map(|loc| loc.display_name().len())
        .max()
        .unwrap_or(8) as u16;
    let content_w = (max_name_len + 4).max(16);
    let content_h = locales.len() as u16;
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(format!(" {} ", crate::i18n::t(l, "settings.language")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let current_locale = crate::i18n::Locale::from_str(&app.config.language);
    let rows: Vec<Row> = locales
        .iter()
        .enumerate()
        .map(|(idx, loc)| {
            let is_selected = idx == app.language_selected;
            let is_current = *loc == current_locale;
            let marker = if is_current { "✓ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            Row::new(vec![Cell::from(format!(
                "{}{}",
                marker,
                loc.display_name()
            ))
            .style(style)])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Min(0)]);
    f.render_widget(table, inner);
}

/// Third-level popup: Provider detail editor, overlaid on top of relay settings
pub fn draw_relay_detail(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let l = app.locale;

    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));

    // Content-fit: use actual URL length to determine width
    let max_url_len = prov
        .map(|p| p.base_url.len().max(p.label.len()))
        .unwrap_or(20) as u16;
    let content_w = (max_url_len.max(30).max(40) * 8 / 5) as u16; // 1.6x width
    let base_lines = 8u16;
    let test_lines = if app.provider_test_in_progress {
        2
    } else if prov.map(|p| p.test_result.is_some()).unwrap_or(false) {
        4
    } else {
        0
    };
    let content_h = (base_lines + test_lines + 1) * 8 / 5; // 1.6x height
    let area = popup_area(content_w, content_h, f.area());

    render_modal_surface(f, area, theme);

    let prov_label = prov.map(|p| p.label.as_str()).unwrap_or("?");
    let block = Block::default()
        .title(format!(
            " {} [{}] ",
            crate::i18n::t(l, "relay.details"),
            prov_label
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3), // leave room for footer
    };

    let detail_lines: Vec<Line> = if let Some(prov) = prov {
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
            if field == idx {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
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
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.label"),
                Style::default().fg(theme.comment),
            )),
            Line::from(Span::styled(make_val(0, &prov.label), field_style(0))),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.base_url"),
                Style::default().fg(theme.comment),
            )),
            Line::from(Span::styled(make_val(1, &prov.base_url), field_style(1))),
            Line::from(""),
            Line::from(Span::styled(
                crate::i18n::t(l, "relay.api_key"),
                Style::default().fg(theme.comment),
            )),
            Line::from(Span::styled(key_display, field_style(2))),
        ];
        if app.provider_test_in_progress {
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
            for line in result.lines().take(4) {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(color),
                )));
            }
        }
        lines
    } else {
        vec![Line::from(Span::styled(
            crate::i18n::t(l, "relay.no_provider"),
            Style::default().fg(theme.comment),
        ))]
    };

    let para = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);

    // Footer
    let footer_text = if app.relay_editing {
        crate::i18n::t(l, "relay.footer_edit")
    } else {
        crate::i18n::t(l, "relay.footer_detail")
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

pub fn draw_agent_style_modal(f: &mut Frame, app: &App) {
    use crate::i18n::t;
    let theme = &app.theme;
    let l = app.locale;
    let style = &app.config.desired_agent_style;

    let zoom_desc = if style.zoom == "auto" {
        "agent_style.desc_zoom_auto"
    } else {
        "agent_style.desc_zoom_keep"
    };
    let status_desc = match style.status.as_str() {
        "show" => "agent_style.desc_status_show",
        "hide" => "agent_style.desc_status_hide",
        _ => "agent_style.desc_status_keep",
    };

    let items: &[(&str, &str, &str)] = &[
        ("agent_style.zoom", &style.zoom, zoom_desc),
        ("agent_style.status", &style.status, status_desc),
    ];

    let area = super::layout::popup_area(62, items.len() as u16 + 4, f.area());
    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(format!(" ✦ {} ", t(l, "agent_style.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    };

    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(idx, (name_key, cur_val, desc_key))| {
            let is_selected = idx == app.agent_style_selected;
            let name_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            let val_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent)
            };

            let val_display = match *cur_val {
                "auto" => t(l, "agent_style.zoom_auto"),
                "show" => t(l, "agent_style.status_show"),
                "hide" => t(l, "agent_style.status_hide"),
                "keep" => {
                    if *name_key == "agent_style.zoom" {
                        t(l, "agent_style.zoom_keep")
                    } else {
                        t(l, "agent_style.status_keep")
                    }
                }
                other => other,
            };

            Row::new(vec![
                Cell::from(t(l, name_key)).style(name_style),
                Cell::from(format!("‹ {} ›", val_display)).style(val_style),
                Cell::from(t(l, desc_key)).style(Style::default().fg(theme.comment)),
            ])
        })
        .collect();

    let table = ratatui::widgets::Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(0),
        ],
    );
    f.render_widget(table, inner);

    let footer = Paragraph::new(t(l, "agent_style.footer"))
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

pub fn draw_telegram_settings_modal(f: &mut Frame, app: &App) {
    use crate::runtime_status;

    let theme = &app.theme;
    let locale = app.locale;
    let area = super::layout::popup_area(72, 13, f.area());
    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(format!(" ✈ Telegram "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(3),
    };

    let enabled_value = if app.config.telegram.enabled {
        crate::i18n::t(locale, "settings.on").to_string()
    } else {
        crate::i18n::t(locale, "settings.off").to_string()
    };
    let token_value = if app.telegram_editing && app.telegram_selected_field == 1 {
        format!("{}|", app.telegram_edit_buffer)
    } else {
        mask_secret(&app.config.telegram.bot_token)
    };
    let chat_value = if app.telegram_editing && app.telegram_selected_field == 2 {
        format!("{}|", app.telegram_edit_buffer)
    } else if app.config.telegram.chat_id.is_empty() {
        "(empty)".to_string()
    } else {
        app.config.telegram.chat_id.clone()
    };
    let username_value = if app.config.telegram.bot_username.is_empty() {
        "(unknown)".to_string()
    } else {
        format!("@{}", app.config.telegram.bot_username)
    };
    let pad_status = runtime_status::describe_status(&crate::paths::pad_status_path());
    let bot_status = runtime_status::describe_status(&crate::paths::telegram_bot_status_path());
    let restart_value = if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    ) {
        "立即重启".to_string()
    } else {
        "Restart now".to_string()
    };

    let rows = vec![
        telegram_row(
            app,
            0,
            &telegram_label(locale, "enabled"),
            &enabled_value,
            true,
        ),
        telegram_row(
            app,
            1,
            &telegram_label(locale, "bot_token"),
            &token_value,
            true,
        ),
        telegram_row(
            app,
            2,
            &telegram_label(locale, "chat_id"),
            &chat_value,
            true,
        ),
        telegram_row(
            app,
            3,
            &telegram_label(locale, "restart_bot"),
            &restart_value,
            true,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "bot_username"),
            &username_value,
            false,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "pad_status"),
            &pad_status,
            false,
        ),
        telegram_row(
            app,
            99,
            &telegram_label(locale, "bot_status"),
            &bot_status,
            false,
        ),
    ];

    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(0)]);
    f.render_widget(table, inner);

    let footer_text = if app.telegram_editing {
        if matches!(
            locale,
            crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
        ) {
            "输入编辑 | Enter: 保存 | Esc: 取消"
        } else {
            "Type to edit | Enter: save | Esc: cancel"
        }
    } else if matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    ) {
        "j/k: 移动 | Enter/Space: 编辑/切换/重启 | r: 重启 | Esc: 返回"
    } else {
        "j/k: move | Enter/Space: edit/toggle/restart | r: restart | Esc: back"
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

fn telegram_row(
    app: &App,
    field_idx: usize,
    name: &str,
    value: &str,
    editable: bool,
) -> Row<'static> {
    let theme = &app.theme;
    let is_selected = editable && field_idx == app.telegram_selected_field;
    let name_style = if is_selected {
        Style::default()
            .bg(theme.highlight_bg)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg)
    };
    let value_style = if is_selected {
        Style::default().bg(theme.highlight_bg).fg(theme.accent)
    } else if editable {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.comment)
    };
    Row::new(vec![
        Cell::from(name.to_string()).style(name_style),
        Cell::from(value.to_string()).style(value_style),
    ])
}

fn telegram_label(locale: crate::i18n::Locale, key: &str) -> String {
    let zh = matches!(
        locale,
        crate::i18n::Locale::ZhCN | crate::i18n::Locale::ZhTW
    );
    match key {
        "enabled" if zh => "启用".to_string(),
        "enabled" => "Enabled".to_string(),
        "bot_token" if zh => "Bot Token".to_string(),
        "bot_token" => "Bot Token".to_string(),
        "chat_id" if zh => "Chat ID".to_string(),
        "chat_id" => "Chat ID".to_string(),
        "restart_bot" if zh => "重启 Bot".to_string(),
        "restart_bot" => "Restart Bot".to_string(),
        "bot_username" if zh => "Bot Username".to_string(),
        "bot_username" => "Bot Username".to_string(),
        "pad_status" if zh => "Pad 状态".to_string(),
        "pad_status" => "Pad Status".to_string(),
        "bot_status" if zh => "Bot 守护进程".to_string(),
        "bot_status" => "Bot Daemon".to_string(),
        _ => key.to_string(),
    }
}

fn mask_secret(secret: &str) -> String {
    if secret.is_empty() {
        return "(empty)".to_string();
    }
    let chars = secret.chars().collect::<Vec<_>>();
    if chars.len() <= 10 {
        return "*".repeat(chars.len());
    }
    let head = chars.iter().take(4).collect::<String>();
    let tail = chars
        .iter()
        .rev()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{}…{}", head, tail)
}
