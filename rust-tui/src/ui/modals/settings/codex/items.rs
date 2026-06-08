use crate::app::state::CodexSettingsView;
use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::SelectionItem;

use super::{categories, options};

pub(super) fn codex_items(app: &App) -> Vec<SelectionItem> {
    match app.codex_settings_view {
        CodexSettingsView::Categories => categories::category_items(app),
        CodexSettingsView::Runtime => options::runtime_items(app),
        CodexSettingsView::StatusLine => options::status_line_items(app),
        CodexSettingsView::Prompts => options::prompt_items(app),
        CodexSettingsView::Preview => options::preview_items(app),
        CodexSettingsView::Cli => options::cli_items(app),
    }
}

pub(super) fn codex_title(app: &App) -> String {
    let base = t(app.locale, "settings.codex_settings");
    match app.codex_settings_view {
        CodexSettingsView::Categories => base.to_string(),
        view => format!(
            "{} / {}",
            base,
            categories::category_title(app, view.category_index())
        ),
    }
}

pub(super) fn codex_footer(app: &App) -> &'static str {
    match app.codex_settings_view {
        CodexSettingsView::Categories => "j/k move · Enter open · Esc back",
        CodexSettingsView::Cli => "j/k move · Enter check · u update · h back · Esc back",
        _ => "j/k move · Enter/Space toggle or cycle · h back · Esc back",
    }
}

pub(super) fn switch_item(
    app: &App,
    name_key: &str,
    enabled: bool,
    desc_key: &str,
) -> SelectionItem {
    value_item(
        app,
        name_key,
        if enabled {
            t(app.locale, "settings.on").to_string()
        } else {
            t(app.locale, "settings.off").to_string()
        },
        desc_key,
    )
}

pub(super) fn value_item(
    app: &App,
    name_key: &str,
    value: String,
    desc_key: &str,
) -> SelectionItem {
    SelectionItem {
        title: t(app.locale, name_key).to_string(),
        value: None,
        subtitle: Some(format!("{}  ·  {}", value, t(app.locale, desc_key))),
        keyword: Some(format!(
            "{} {} {}",
            t(app.locale, name_key),
            value,
            t(app.locale, desc_key)
        )),
        disabled: false,
    }
}

pub(super) fn on_off(app: &App, enabled: bool) -> &'static str {
    if enabled {
        t(app.locale, "settings.on")
    } else {
        t(app.locale, "settings.off")
    }
}

pub(super) fn web_search_label(app: &App) -> String {
    t(
        app.locale,
        match app.config.codex.web_search.as_str() {
            "cached" => "settings.codex_web_search_cached",
            "live" => "settings.codex_web_search_live",
            "disabled" => "settings.codex_web_search_disabled",
            _ => "settings.codex_web_search_default",
        },
    )
    .to_string()
}
