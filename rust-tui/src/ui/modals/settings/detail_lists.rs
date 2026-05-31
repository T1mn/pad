use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(super) fn draw_theme_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
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
                value: None,
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
        &format!("{} [{}]", t(locale, "settings.theme"), app.theme.name),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

pub(super) fn draw_language_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let current_locale = crate::i18n::Locale::from_str(&app.config.language);
    let items: Vec<SelectionItem> = App::available_locales()
        .iter()
        .map(|entry| {
            let is_current = *entry == current_locale;
            SelectionItem {
                title: if is_current {
                    format!("✓ {}", entry.display_name())
                } else {
                    entry.display_name().to_string()
                },
                value: None,
                subtitle: Some(entry.as_str().to_string()),
                keyword: Some(format!("{} {}", entry.display_name(), entry.as_str())),
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
        t(locale, "settings.language"),
        &items,
        &state,
        Some("j/k move · Enter apply · Esc back"),
    );
}

pub(super) fn draw_agent_style_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
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
            "auto" => t(locale, "agent_style.zoom_auto"),
            "show" => t(locale, "agent_style.status_show"),
            "hide" => t(locale, "agent_style.status_hide"),
            "keep" => {
                if *name_key == "agent_style.zoom" {
                    t(locale, "agent_style.zoom_keep")
                } else {
                    t(locale, "agent_style.status_keep")
                }
            }
            other => other,
        };
        SelectionItem {
            title: t(locale, name_key).to_string(),
            value: None,
            subtitle: Some(format!("{}  ·  {}", val_display, t(locale, desc_key))),
            keyword: Some(format!(
                "{} {} {}",
                t(locale, name_key),
                val_display,
                t(locale, desc_key)
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
        t(locale, "agent_style.title"),
        &items,
        &state,
        Some("j/k move · Enter/Space toggle · Esc back"),
    );
}

pub(super) fn draw_sound_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let locale = app.locale;
    let sound = &app.config.sound;
    let preset_name = |preset_id: &str| {
        crate::sound::preset(preset_id)
            .map(|preset| t(locale, preset.name_key).to_string())
            .unwrap_or_else(|| preset_id.to_string())
    };
    let preset_desc = |preset_id: &str| {
        crate::sound::preset(preset_id)
            .map(|preset| t(locale, preset.desc_key).to_string())
            .unwrap_or_else(|| t(locale, "sound.preset_desc").to_string())
    };
    let items: Vec<SelectionItem> = [
        (
            t(locale, "sound.enabled").to_string(),
            if sound.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.enabled_desc").to_string(),
        ),
        (
            t(locale, "sound.event.completion").to_string(),
            if sound.completion.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.completion_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.completion.preset),
            preset_desc(&sound.completion.preset),
        ),
        (
            t(locale, "sound.event.approval").to_string(),
            if sound.approval.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.approval_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.approval.preset),
            preset_desc(&sound.approval.preset),
        ),
        (
            t(locale, "sound.event.timeout").to_string(),
            if sound.timeout.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.timeout_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.timeout.preset),
            preset_desc(&sound.timeout.preset),
        ),
        (
            t(locale, "sound.event.failure").to_string(),
            if sound.failure.enabled {
                t(locale, "settings.on").to_string()
            } else {
                t(locale, "settings.off").to_string()
            },
            t(locale, "sound.event.failure_desc").to_string(),
        ),
        (
            t(locale, "sound.preset").to_string(),
            preset_name(&sound.failure.preset),
            preset_desc(&sound.failure.preset),
        ),
    ]
    .iter()
    .map(|(name, value, desc)| SelectionItem {
        title: name.clone(),
        value: None,
        subtitle: Some(format!("{value}  ·  {desc}")),
        keyword: Some(format!("{name} {value} {desc}")),
        detail: None,
        disabled: false,
    })
    .collect();

    let mut state = SelectionState {
        selected: app.sound_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        t(locale, "settings.sound"),
        &items,
        &state,
        Some("j/k move · Enter toggle/cycle · Space preview/toggle · Esc back"),
    );
}
