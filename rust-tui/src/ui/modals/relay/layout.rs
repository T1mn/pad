use crate::app::App;
use crate::i18n::Locale;
use crate::theme::Theme;
use ratatui::style::Color;

pub(super) fn relay_detail_width(app: &App) -> u16 {
    match app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
    {
        Some("codex") => 82,
        Some("opencode") => 78,
        _ => 68,
    }
}

pub(super) fn relay_detail_base_lines(app: &App) -> u16 {
    match app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
    {
        Some("codex") => 14,
        Some("claude") => 17,
        Some("opencode") => 22,
        _ => 14,
    }
}

pub(super) fn relay_provider_footer_text<'a>(app: &App, locale: Locale) -> &'a str {
    match app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str())
    {
        Some("claude" | "codex") => crate::i18n::t(locale, "relay.footer_provider_codex"),
        Some("opencode") => crate::i18n::t(locale, "relay.footer_provider_opencode"),
        _ => crate::i18n::t(locale, "relay.footer_provider"),
    }
}

pub(super) fn relay_detail_footer_text<'a>(app: &App, locale: Locale) -> &'a str {
    if app.relay_popup_mode != crate::app::state::RelayPopupMode::None {
        match app.relay_popup_mode {
            crate::app::state::RelayPopupMode::OpenCodeModels => {
                if app.relay_popup_editing {
                    crate::i18n::t(locale, "relay.footer_edit")
                } else {
                    crate::i18n::t(locale, "relay.footer_models")
                }
            }
            crate::app::state::RelayPopupMode::OpenCodeDefaultModel
            | crate::app::state::RelayPopupMode::OpenCodeSmallModel => {
                crate::i18n::t(locale, "relay.footer_model_picker")
            }
            crate::app::state::RelayPopupMode::None => {
                crate::i18n::t(locale, "relay.footer_detail")
            }
        }
    } else if app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str() == "codex")
        .unwrap_or(false)
    {
        crate::i18n::t(locale, "relay.footer_detail_codex")
    } else if app
        .config
        .agents
        .get(app.relay_selected_agent)
        .map(|agent| agent.name.as_str() == "opencode")
        .unwrap_or(false)
    {
        crate::i18n::t(locale, "relay.footer_detail_opencode")
    } else {
        crate::i18n::t(locale, "relay.footer_detail")
    }
}

pub(super) fn yes_no(ready: bool) -> &'static str {
    if ready {
        "ready"
    } else {
        "missing"
    }
}

pub(super) fn http_status_color(status: u16, theme: &Theme) -> Color {
    match status {
        100..=399 => theme.success,
        400..=499 => theme.warning,
        500..=599 => theme.error,
        _ => theme.comment,
    }
}

pub(super) fn latency_color(latency_ms: u64, theme: &Theme) -> Color {
    match latency_ms {
        0..=800 => theme.success,
        801..=2500 => theme.warning,
        _ => theme.error,
    }
}
