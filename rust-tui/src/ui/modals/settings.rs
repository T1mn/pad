use super::common::render_modal_surface;
use crate::app::App;
use crate::i18n::t;
use crate::tree::AgentLauncher;
use crate::ui::layout::popup_area;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

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
        .border_type(BorderType::Rounded)
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
    let content_h = themes.len() as u16 + 2;
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
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.keyword));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width - 4,
        height: area.height - 2,
    };

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
        .border_type(BorderType::Rounded)
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

pub fn draw_agent_style_modal(f: &mut Frame, app: &App) {
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

    let area = popup_area(62, items.len() as u16 + 4, f.area());
    render_modal_surface(f, area, theme);

    let block = Block::default()
        .title(format!(" ✦ {} ", t(l, "agent_style.title")))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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

    let table = Table::new(
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
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let table = Table::new(items, [Constraint::Percentage(100)]).block(block);

    f.render_widget(table, popup_area);
}
