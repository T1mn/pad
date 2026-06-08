use crate::app::App;
use crate::i18n::t;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(in crate::ui::modals::settings) fn draw_agent_style_detail(
    f: &mut Frame,
    app: &App,
    area: Rect,
) {
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
