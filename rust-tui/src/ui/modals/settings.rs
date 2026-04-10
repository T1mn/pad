use super::common::render_modal_surface;
use super::relay::draw_relay_in_area;
use super::telegram::draw_telegram_settings_content;
use crate::app::state::SettingsDetailKind;
use crate::app::App;
use crate::i18n::{t, Locale};
use crate::tree::AgentLauncher;
use crate::ui::layout::popup_area;
use crate::ui::selection::{
    render::{recommended_list_modal_height, render_selection_surface},
    SelectionItem, SelectionState,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let (content_w, content_h) = settings_modal_size(app);
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    if app.settings_focus == crate::app::state::SettingsFocus::Detail && !app.settings_searching {
        draw_settings_detail_panel(f, app, inner);
    } else {
        draw_settings_list(f, app, inner);
    }
}

fn draw_settings_list(f: &mut Frame, app: &App, area: Rect) {
    let l = app.locale;
    let items = app.filtered_settings_items();
    let selection_items: Vec<SelectionItem> = items
        .iter()
        .map(|(id, value, name_key, desc_key, editable)| SelectionItem {
            title: crate::i18n::t(l, name_key).to_string(),
            subtitle: Some(if *editable {
                format!("{}  ›", value)
            } else {
                value.clone()
            }),
            keyword: Some(crate::app::actions::settings_item_search_blob(
                l, id, value, name_key, desc_key,
            )),
            detail: None,
            disabled: false,
        })
        .collect();
    let mut state = SelectionState {
        selected: app.settings_selected,
        scroll: 0,
        query: app.settings_search.clone(),
        searching: app.settings_searching,
    };
    state.clamp_selected(selection_items.len());
    render_selection_surface(
        f,
        area,
        &app.theme,
        crate::i18n::t(l, "settings.title"),
        &selection_items,
        &state,
        Some(settings_list_footer(app.locale)),
    );
}

fn settings_modal_size(app: &App) -> (u16, u16) {
    if app.settings_focus == crate::app::state::SettingsFocus::Detail && !app.settings_searching {
        settings_detail_modal_size(app)
    } else {
        settings_list_modal_size(app)
    }
}

fn settings_list_modal_size(app: &App) -> (u16, u16) {
    let l = app.locale;
    let items = app.filtered_settings_items();
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
    let content_w = (max_name + max_value + 26).clamp(48, 72);
    let row_count = items.len().max(1) as u16;
    let content_h = recommended_list_modal_height(row_count, 2, 1, 1).clamp(12, 22);
    (content_w, content_h)
}

fn settings_detail_modal_size(app: &App) -> (u16, u16) {
    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Theme) => {
            let row_count = App::available_themes().len().max(1) as u16;
            (
                58,
                recommended_list_modal_height(row_count, 2, 1, 1).clamp(16, 24),
            )
        }
        Some(SettingsDetailKind::Language) => {
            let row_count = App::available_locales().len().max(1) as u16;
            (
                56,
                recommended_list_modal_height(row_count, 2, 1, 1).clamp(12, 18),
            )
        }
        Some(SettingsDetailKind::AgentStyle) => (64, 12),
        Some(SettingsDetailKind::CodexSettings) => (76, 15),
        Some(SettingsDetailKind::AutoRefresh)
        | Some(SettingsDetailKind::ClaudeFullAccess)
        | Some(SettingsDetailKind::PreviewMode)
        | Some(SettingsDetailKind::DisplayMode)
        | Some(SettingsDetailKind::Version) => (60, 10),
        Some(SettingsDetailKind::Relay) => relay_modal_size(app),
        Some(SettingsDetailKind::Telegram) => (72, 13),
        None => settings_list_modal_size(app),
    }
}

fn relay_modal_size(app: &App) -> (u16, u16) {
    let selected_agent = app.config.agents.get(app.relay_selected_agent);
    let agent_name = selected_agent.map(|agent| agent.name.as_str());
    match app.relay_view {
        crate::app::state::RelayView::AgentList => {
            let row_count = app.config.agents.len().max(1) as u16;
            (
                58,
                recommended_list_modal_height(row_count, 2, 1, 1).max(12),
            )
        }
        crate::app::state::RelayView::ProviderList => {
            let row_count = selected_agent
                .map(|a| a.providers.len().max(1) as u16)
                .unwrap_or(1);
            (
                76,
                recommended_list_modal_height(row_count, 2, 1, 1).max(12),
            )
        }
        crate::app::state::RelayView::DetailPane => {
            let prov = selected_agent.and_then(|a| a.providers.get(app.relay_selected_provider));
            let base_lines = match agent_name {
                Some("codex") => 18u16,
                Some("opencode") => 22u16,
                _ => 14u16,
            };
            let test_lines = if app.provider_test_in_progress {
                2
            } else if prov.map(|p| p.test_result.is_some()).unwrap_or(false) {
                4
            } else {
                0
            };
            (
                match agent_name {
                    Some("codex") => 82,
                    Some("opencode") => 78,
                    _ => 68,
                },
                base_lines + test_lines,
            )
        }
    }
}

fn draw_settings_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let Some(kind) = app.current_settings_detail_kind() else {
        return;
    };

    match kind {
        SettingsDetailKind::Theme => draw_theme_detail(f, app, area),
        SettingsDetailKind::AutoRefresh => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.auto_refresh"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::CodexSettings => draw_codex_detail(f, app, area),
        SettingsDetailKind::ClaudeFullAccess => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.claude_full_access"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::Relay => draw_relay_in_area(f, app, area),
        SettingsDetailKind::Telegram => draw_telegram_detail(f, app, area),
        SettingsDetailKind::AgentStyle => draw_agent_style_detail(f, app, area),
        SettingsDetailKind::PreviewMode => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.preview_mode"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space cycle · Esc back",
        ),
        SettingsDetailKind::DisplayMode => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.display_mode"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Enter/Space toggle · Esc back",
        ),
        SettingsDetailKind::Language => draw_language_detail(f, app, area),
        SettingsDetailKind::Version => draw_simple_detail(
            f,
            app,
            area,
            t(app.locale, "settings.version"),
            simple_value_line(app, kind),
            vec![detail_body_line(app.locale, kind)],
            "Read only · Esc back",
        ),
    }
}

fn draw_theme_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let items: Vec<SelectionItem> = App::available_themes()
        .iter()
        .map(|(name, desc)| {
            let is_current = *name == app.config.theme;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", name)
                } else {
                    name.to_string()
                },
                subtitle: Some(if is_current {
                    format!("{}  ·  current", desc)
                } else {
                    (*desc).to_string()
                }),
                keyword: Some(format!("{} {}", name, desc)),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.theme_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        &format!("{} [{}]", t(l, "settings.theme"), app.theme.name),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

fn draw_language_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let current_locale = crate::i18n::Locale::from_str(&app.config.language);
    let items: Vec<SelectionItem> = App::available_locales()
        .iter()
        .map(|loc| {
            let is_current = *loc == current_locale;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", loc.display_name())
                } else {
                    loc.display_name().to_string()
                },
                subtitle: Some(loc.as_str().to_string()),
                keyword: Some(format!("{} {}", loc.display_name(), loc.as_str())),
                detail: None,
                disabled: false,
            }
        })
        .collect();
    let mut state = SelectionState {
        selected: app.language_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(l, "settings.language"),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

fn draw_agent_style_detail(f: &mut Frame, app: &App, area: Rect) {
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
    let items: Vec<SelectionItem> = [
        ("agent_style.zoom", style.zoom.as_str(), zoom_desc),
        ("agent_style.status", style.status.as_str(), status_desc),
    ]
    .iter()
    .map(|(name_key, cur_val, desc_key)| {
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
        SelectionItem {
            title: t(l, name_key).to_string(),
            subtitle: Some(format!("{}  ·  {}", val_display, t(l, desc_key))),
            keyword: Some(format!(
                "{} {} {}",
                t(l, name_key),
                val_display,
                t(l, desc_key)
            )),
            detail: None,
            disabled: false,
        }
    })
    .collect();
    let mut state = SelectionState {
        selected: app.agent_style_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(l, "agent_style.title"),
        &items,
        &state,
        Some("j/k move · Enter/Space toggle · Esc back"),
    );
}

fn draw_codex_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let l = app.locale;
    let items: Vec<SelectionItem> = [
        (
            "settings.codex_yolo",
            if app.config.agent_permissions.codex_auto_full_access {
                t(l, "settings.on")
            } else {
                t(l, "settings.off")
            },
            "settings.codex_yolo_desc",
        ),
        (
            "settings.codex_fast",
            if app.config.codex.fast_mode {
                t(l, "settings.on")
            } else {
                t(l, "settings.off")
            },
            "settings.codex_fast_desc",
        ),
        (
            "settings.codex_multi_agent",
            if app.config.codex.multi_agent {
                t(l, "settings.on")
            } else {
                t(l, "settings.off")
            },
            "settings.codex_multi_agent_desc",
        ),
        (
            "settings.codex_web_search",
            t(
                l,
                match app.config.codex.web_search.as_str() {
                    "cached" => "settings.codex_web_search_cached",
                    "live" => "settings.codex_web_search_live",
                    "disabled" => "settings.codex_web_search_disabled",
                    _ => "settings.codex_web_search_default",
                },
            ),
            "settings.codex_web_search_desc",
        ),
    ]
    .iter()
    .map(|(name_key, value, desc_key)| SelectionItem {
        title: t(l, name_key).to_string(),
        subtitle: Some(format!("{}  ·  {}", value, t(l, desc_key))),
        keyword: Some(format!("{} {} {}", t(l, name_key), value, t(l, desc_key))),
        detail: None,
        disabled: false,
    })
    .collect();
    let mut state = SelectionState {
        selected: app.codex_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(l, "settings.codex_settings"),
        &items,
        &state,
        Some("j/k move · Enter/Space toggle or cycle · Esc back"),
    );
}

fn draw_telegram_detail(f: &mut Frame, app: &App, area: Rect) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Telegram",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );
    draw_telegram_settings_content(f, app, body_area, false);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "j/k move · Enter/Space edit · r restart · Esc back",
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        footer_area,
    );
}

fn draw_simple_detail(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    value: String,
    body: Vec<String>,
    footer: &str,
) {
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ))),
        header_area,
    );

    let mut lines = vec![
        Line::from(Span::styled(
            value,
            Style::default()
                .fg(app.theme.highlight_fg)
                .bg(app.theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
    ];
    lines.extend(body.into_iter().map(|line| {
        Line::from(Span::styled(
            line,
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))
    }));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            footer.to_string(),
            Style::default()
                .fg(app.theme.comment)
                .add_modifier(Modifier::DIM),
        ))),
        footer_area,
    );
}

fn simple_value_line(app: &App, kind: SettingsDetailKind) -> String {
    let l = app.locale;
    match kind {
        SettingsDetailKind::AutoRefresh => {
            if app.config.auto_refresh {
                t(l, "settings.on").to_string()
            } else {
                t(l, "settings.off").to_string()
            }
        }
        SettingsDetailKind::ClaudeFullAccess => {
            if app.config.agent_permissions.claude_auto_full_access {
                t(l, "settings.on").to_string()
            } else {
                t(l, "settings.off").to_string()
            }
        }
        SettingsDetailKind::PreviewMode => match app.config.preview.mode.as_str() {
            "tmux" => t(l, "settings.preview_mode_tmux").to_string(),
            "session" => t(l, "settings.preview_mode_session").to_string(),
            _ => t(l, "settings.preview_mode_auto").to_string(),
        },
        SettingsDetailKind::DisplayMode => match app.config.display.session_scope.as_str() {
            "all" => t(l, "settings.display_mode_all").to_string(),
            _ => t(l, "settings.display_mode_live").to_string(),
        },
        SettingsDetailKind::Version => env!("CARGO_PKG_VERSION").to_string(),
        _ => String::new(),
    }
}

fn detail_body_line(locale: Locale, kind: SettingsDetailKind) -> String {
    match (locale, kind) {
        (Locale::ZhCN, SettingsDetailKind::AutoRefresh) => "控制面板扫描是否自动刷新".to_string(),
        (Locale::ZhCN, SettingsDetailKind::ClaudeFullAccess) => {
            "启动时自动植入 bypassPermissions，并关闭 Claude sandbox".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::PreviewMode) => {
            "切换预览读取来源：自动 / tmux / session".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::DisplayMode) => {
            "切换只显示 live session 或显示全部 session".to_string()
        }
        (Locale::ZhCN, SettingsDetailKind::Version) => "当前 pad 版本".to_string(),
        (_, SettingsDetailKind::AutoRefresh) => {
            "Controls whether pad refreshes scans automatically.".to_string()
        }
        (_, SettingsDetailKind::ClaudeFullAccess) => {
            "Apply bypassPermissions and disable Claude sandbox before launch.".to_string()
        }
        (_, SettingsDetailKind::PreviewMode) => {
            "Switch preview source between auto, tmux pane, and session transcript.".to_string()
        }
        (_, SettingsDetailKind::DisplayMode) => {
            "Switch between live-only sessions and all sessions.".to_string()
        }
        (_, SettingsDetailKind::Version) => "Current pad version.".to_string(),
        _ => String::new(),
    }
}

fn settings_list_footer(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "↑/↓ 或 j/k 移动 · Enter 打开 · / 搜索 · Esc 关闭",
        Locale::ZhTW => "↑/↓ 或 j/k 移動 · Enter 打開 · / 搜尋 · Esc 關閉",
        _ => "↑/↓ or j/k move · Enter open · / search · Esc close",
    }
}

pub fn draw_theme_selector(f: &mut Frame, app: &App) {
    draw_settings_modal(f, app);
}

pub fn draw_language_selector(f: &mut Frame, app: &App) {
    draw_settings_modal(f, app);
}

pub fn draw_agent_style_modal(f: &mut Frame, app: &App) {
    draw_settings_modal(f, app);
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
        .title_alignment(ratatui::layout::Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(theme.bg).fg(theme.fg))
        .border_style(Style::default().fg(theme.accent));

    let table = Table::new(items, [Constraint::Percentage(100)]).block(block);

    f.render_widget(table, popup_area);
}

#[cfg(test)]
mod tests {
    use crate::i18n::Locale;

    #[test]
    fn settings_selection_keyword_includes_english_aliases() {
        let keyword = crate::app::actions::settings_item_search_blob(
            Locale::ZhCN,
            "relay",
            "配置",
            "settings.relay",
            "settings.relay",
        );
        assert!(keyword.contains("relay"));
        assert!(keyword.contains("provider"));
    }
}
