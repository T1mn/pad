mod edit;
mod models;
mod picker;

use super::super::common::render_modal_surface;
use crate::app::state::RelayPopupMode;
use crate::app::App;
use crate::ui::layout::popup_area;
use ratatui::{layout::Rect, Frame};

pub(super) fn draw_relay_popup(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_rect = match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => popup_area(60, 16, area),
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            popup_area(66, 14, area)
        }
        RelayPopupMode::None => return,
    };
    render_modal_surface(f, popup_rect, theme);

    match app.relay_popup_mode {
        RelayPopupMode::OpenCodeModels => {
            models::draw_models_popup(f, app, popup_rect, popup_inner(popup_rect));
        }
        RelayPopupMode::OpenCodeDefaultModel | RelayPopupMode::OpenCodeSmallModel => {
            picker::draw_model_picker_popup(f, app, popup_rect);
        }
        RelayPopupMode::None => {}
    }
}

fn popup_inner(popup_rect: Rect) -> Rect {
    Rect {
        x: popup_rect.x + 2,
        y: popup_rect.y + 1,
        width: popup_rect.width.saturating_sub(4),
        height: popup_rect.height.saturating_sub(2),
    }
}
