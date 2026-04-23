mod detail;
mod detail_lists;
mod launcher;
mod layout;
mod list;

use super::common::render_modal_surface;
use crate::app::App;
use crate::ui::layout::popup_area;
use ratatui::{layout::Rect, Frame};

pub fn draw_settings_modal(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let (content_w, content_h) = layout::settings_modal_size(app);
    let area = popup_area(content_w, content_h, f.area());
    render_modal_surface(f, area, theme);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    if app.settings_focus == crate::app::state::SettingsFocus::Detail && !app.settings_searching {
        detail::draw_settings_detail_panel(f, app, inner);
    } else {
        list::draw_settings_list(f, app, inner);
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

pub fn draw_agent_launcher(
    f: &mut Frame,
    app: &App,
    launcher: &crate::tree::AgentLauncher,
    area: Rect,
) {
    launcher::draw_agent_launcher(f, app, launcher, area);
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
