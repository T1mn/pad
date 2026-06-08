use crate::i18n::{t, Locale};
use crate::theme::Config;

pub(super) fn toggle_label(locale: Locale, enabled: bool) -> String {
    t(
        locale,
        if enabled {
            "settings.on"
        } else {
            "settings.off"
        },
    )
    .to_string()
}

pub(super) fn preview_mode_label(config: &Config, locale: Locale) -> String {
    let key = match config.preview.mode.as_str() {
        "tmux" => "settings.preview_mode_tmux",
        "session" => "settings.preview_mode_session",
        _ => "settings.preview_mode_auto",
    };
    t(locale, key).to_string()
}

pub(super) fn display_mode_label(config: &Config, locale: Locale) -> String {
    let key = match config.display.session_scope.as_str() {
        "all" => "settings.display_mode_all",
        _ => "settings.display_mode_live",
    };
    t(locale, key).to_string()
}

pub(super) fn sound_summary(config: &Config, locale: Locale) -> String {
    let enabled_events = [
        config.sound.completion.enabled,
        config.sound.approval.enabled,
        config.sound.timeout.enabled,
        config.sound.failure.enabled,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();

    if config.sound.enabled {
        format!("{} · {enabled_events}/4", t(locale, "settings.on"))
    } else {
        t(locale, "settings.off").to_string()
    }
}

pub(super) fn codex_summary(config: &Config, locale: Locale) -> String {
    format!(
        "YOLO {}  ·  Fast {}  ·  Goal {}  ·  MA {}  ·  Web {}  ·  SL {}/3  ·  Sum {}",
        on_off(locale, config.agent_permissions.codex_auto_full_access),
        on_off(locale, config.codex.fast_mode),
        on_off(locale, config.codex.goals),
        on_off(locale, config.codex.multi_agent),
        t(
            locale,
            codex_web_search_key(config.codex.web_search.as_str())
        ),
        config.codex.status_line_items().len(),
        on_off(locale, config.codex.title_summary)
    )
}

fn on_off(locale: Locale, enabled: bool) -> &'static str {
    t(
        locale,
        if enabled {
            "settings.on"
        } else {
            "settings.off"
        },
    )
}

fn codex_web_search_key(value: &str) -> &'static str {
    match value {
        "cached" => "settings.codex_web_search_cached",
        "live" => "settings.codex_web_search_live",
        "disabled" => "settings.codex_web_search_disabled",
        _ => "settings.codex_web_search_default",
    }
}
