use crate::app::App;
use crate::i18n::Locale;
use crate::ui::selection::{render::render_selection_surface, SelectionItem, SelectionState};
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_settings_list(f: &mut Frame, app: &App, area: Rect) {
    let locale = app.locale;
    let items = app.filtered_settings_items();
    let selection_items: Vec<SelectionItem> = items
        .iter()
        .map(|(id, value, name_key, desc_key, editable)| SelectionItem {
            title: crate::i18n::t(locale, name_key).to_string(),
            subtitle: Some(if *editable {
                format!("{}  ›", value)
            } else {
                value.clone()
            }),
            keyword: Some(crate::app::actions::settings_item_search_blob(
                locale, id, value, name_key, desc_key,
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
        crate::i18n::t(locale, "settings.title"),
        &selection_items,
        &state,
        Some(settings_list_footer(app.locale)),
    );
}

pub(super) fn settings_list_footer(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "↑/↓ 或 j/k 移动 · Enter 打开 · / 搜索 · Esc 关闭",
        Locale::ZhTW => "↑/↓ 或 j/k 移動 · Enter 打開 · / 搜尋 · Esc 關閉",
        _ => "↑/↓ or j/k move · Enter open · / search · Esc close",
    }
}
