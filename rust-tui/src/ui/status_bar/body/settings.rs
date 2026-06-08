use crate::app::state::{RelayPopupMode, RelayView, SettingsDetailKind, SettingsFocus};
use crate::app::App;
use crate::i18n::{t, Locale};

pub(super) fn search_status_body(app: &App) -> String {
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

pub(super) fn settings_search_status_body(app: &App) -> String {
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

pub(super) fn settings_status_body(app: &App) -> String {
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
