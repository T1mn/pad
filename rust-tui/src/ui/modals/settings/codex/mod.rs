mod categories;
mod items;
mod options;
mod version;

use crate::app::App;
use crate::ui::selection::{render::render_selection_surface, SelectionState};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(super) fn draw_codex_detail(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let items = items::codex_items(app);
    let mut state = SelectionState {
        selected: app.codex_settings_selected,
        ..Default::default()
    };
    state.clamp_selected(items.len());
    render_selection_surface(
        f,
        area,
        theme,
        &items::codex_title(app),
        &items,
        &state,
        Some(items::codex_footer(app)),
    );
}
