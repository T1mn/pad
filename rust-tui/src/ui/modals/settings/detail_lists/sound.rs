use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(in crate::ui::modals::settings) fn draw_sound_detail(f: &mut Frame, app: &App, area: Rect) {
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
