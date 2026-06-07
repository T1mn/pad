use super::text::{format_two_sided, mode_badge};
use crate::app::state::{Mode, RelayPopupMode, RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crate::i18n::{t, Locale};
use crate::model::PreviewSource;
use ratatui::text::Span;

pub(super) fn mode_span(app: &App) -> Span<'static> {
    let theme = &app.theme;
    let l = app.locale;
    match app.mode {
        Mode::Search => mode_badge(t(l, "mode.search"), theme.mode_search_bg),
        Mode::Settings => mode_badge(t(l, "mode.settings"), theme.accent),
        Mode::TelegramSettings => mode_badge(t(l, "mode.settings"), theme.accent),
        Mode::ThemeSelector => mode_badge(t(l, "mode.theme"), theme.keyword),
        Mode::Help => mode_badge(t(l, "mode.help"), theme.accent),
        Mode::NotificationInbox => mode_badge("INBOX", theme.accent),
        Mode::FilePreview => mode_badge(t(l, "mode.preview"), theme.mode_tree_bg),
        _ if app.sidebar.show_tree => mode_badge(t(l, "mode.tree"), theme.mode_tree_bg),
        _ => mode_badge(t(l, "mode.normal"), theme.mode_normal_bg),
    }
}

pub(super) fn status_body(app: &App, body_width: u16) -> String {
    let l = app.locale;
    match app.mode {
        Mode::Search => search_status_body(app),
        Mode::Settings => {
            if app.settings_searching {
                settings_search_status_body(app)
            } else {
                settings_status_body(app)
            }
        }
        Mode::TelegramSettings => String::from(t(l, "status.settings_nav")),
        Mode::ThemeSelector => String::from(t(l, "status.theme_nav")),
        Mode::Help => String::from(t(l, "status.help_close")),
        Mode::NotificationInbox => {
            "j/k move | Enter/m read | a all read | d delete | Esc close".to_string()
        }
        Mode::FilePreview => String::from(t(l, "status.preview_nav")),
        _ => compose_status_body(app, body_width),
    }
}

fn search_status_body(app: &App) -> String {
    let l = app.locale;
    let clear_hint = if is_zh(l) {
        "Shift+Delete 清空"
    } else {
        "Shift+Delete clear"
    };
    format!(
        "{}: {}  Enter {}  {}  Esc {}",
        t(l, "status.search"),
        app.search_query,
        t(l, "status.confirm"),
        clear_hint,
        t(l, "status.cancel")
    )
}

fn settings_search_status_body(app: &App) -> String {
    if is_zh(app.locale) {
        format!(
            "{}: {}  ↑/↓ 移动  Enter 打开  Shift+Delete 清空  Esc 返回",
            t(app.locale, "status.search"),
            app.settings_search
        )
    } else {
        format!(
            "{}: {}  ↑/↓ move  Enter open  Shift+Delete clear  Esc back",
            t(app.locale, "status.search"),
            app.settings_search
        )
    }
}

fn settings_status_body(app: &App) -> String {
    if app.settings_focus == SettingsFocus::List {
        return if is_zh(app.locale) {
            "↑/↓/j/k: 移动 | Enter: 打开 | /: 搜索 | Esc: 关闭".to_string()
        } else {
            "↑/↓/j/k: move | Enter: open | /: search | Esc: close".to_string()
        };
    }

    match app.current_settings_detail_kind() {
        Some(SettingsDetailKind::Relay) => relay_status_body(app),
        Some(SettingsDetailKind::Telegram) => {
            if is_zh(app.locale) {
                "j/k: 移动 | Enter/Space: 编辑/切换 | r: 重启 | Esc: 返回列表".to_string()
            } else {
                "j/k: move | Enter/Space: edit/toggle | r: restart | Esc: back".to_string()
            }
        }
        Some(SettingsDetailKind::Theme) => {
            if is_zh(app.locale) {
                "j/k: 移动 | Enter: 应用 | Esc: 返回列表".to_string()
            } else {
                "j/k: move | Enter: apply | Esc: back".to_string()
            }
        }
        _ => String::from(t(app.locale, "status.settings_nav")),
    }
}

fn relay_status_body(app: &App) -> String {
    if app.relay_popup_editing || app.relay_editing {
        return t(app.locale, "relay.footer_edit").to_string();
    }
    if app.relay_popup_mode != RelayPopupMode::None {
        return match app.relay_popup_mode {
            RelayPopupMode::OpenCodeModels => t(app.locale, "relay.footer_models").to_string(),
            RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
                t(app.locale, "relay.footer_model_picker").to_string()
            }
            RelayPopupMode::None => t(app.locale, "relay.footer_detail").to_string(),
        };
    }

    match app.relay_view {
        RelayView::AgentList => t(app.locale, "relay.footer_agent").to_string(),
        RelayView::ProviderList => {
            if selected_agent_is(app, "opencode") {
                t(app.locale, "relay.footer_provider_opencode").to_string()
            } else {
                t(app.locale, "relay.footer_provider").to_string()
            }
        }
        RelayView::DetailPane => {
            if selected_agent_is(app, "codex") {
                t(app.locale, "relay.footer_detail_codex").to_string()
            } else if selected_agent_is(app, "opencode") {
                t(app.locale, "relay.footer_detail_opencode").to_string()
            } else {
                t(app.locale, "relay.footer_detail").to_string()
            }
        }
    }
}

pub(super) fn compose_status_body(app: &App, width: u16) -> String {
    let l = app.locale;
    let elapsed = app.last_refresh.elapsed().as_secs();
    let scan_status = if app.scan_in_progress {
        format!(" {}", t(l, "status.scanning"))
    } else {
        String::new()
    };
    let left = if app.sidebar.show_tree {
        if let Some(path) = &app.preview.file_preview_path {
            format!("📁 {}", path.display())
        } else {
            t(l, "tree.explorer").to_string()
        }
    } else {
        format!("{}s {}{}", elapsed, t(l, "status.ago"), scan_status)
    };

    let right_hint = if app.sidebar.show_tree {
        t(l, "status.tree_nav")
    } else if app.preview_is_focused() {
        if app.preview.source == PreviewSource::Session && !app.preview.turns.is_empty() {
            t(l, "status.preview_session_nav")
        } else {
            t(l, "status.preview_nav")
        }
    } else {
        t(l, "status.normal_nav_panel")
    };

    let left_text = if app.sidebar.show_tree {
        format!(
            "{}  {}s {}{}",
            left,
            elapsed,
            t(l, "status.ago"),
            scan_status
        )
    } else {
        format!(" {}", left)
    };

    format_two_sided(&left_text, right_hint, width as usize)
}

fn selected_agent_is(app: &App, name: &str) -> bool {
    app.config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str() == name)
        .unwrap_or(false)
}

fn is_zh(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW)
}
