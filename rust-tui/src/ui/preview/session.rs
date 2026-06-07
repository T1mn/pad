mod badges;
mod detail;
mod list;
mod scroll;
mod text;

use crate::app::App;
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};

pub(crate) use badges::{
    fixed_label, localized_status_label, preview_agent_badge_colors, preview_badge,
};
pub use detail::render_session_detail_lines;
pub(super) use list::render_session_gap_line;
pub(crate) use list::{render_session_card, session_list_total_lines, session_turn_index_at_line};
pub(crate) use scroll::{
    resolve_preview_scroll_for_line_count, resolve_session_list_scroll, visible_detail_window,
};

#[cfg(test)]
pub(crate) use list::build_session_list_lines;

pub(crate) const SESSION_ITEM_CONTENT_HEIGHT: usize = 3;
pub(crate) const SESSION_ITEM_GAP_HEIGHT: usize = 1;

pub(crate) fn draw_session_preview(f: &mut Frame, app: &mut App, area: Rect, theme: &Theme) {
    if app.preview.view == crate::model::PreviewView::SessionDetail {
        if let Some(selected) = app.preview.expanded_turn {
            detail::draw_session_detail(f, app, area, theme, selected);
            return;
        }
    }
    if app.preview.view == crate::model::PreviewView::SessionList
        || app.preview.view == crate::model::PreviewView::SessionDetail
    {
        list::draw_session_list(f, app, area, theme);
    } else if let Some(selected) = app.preview.expanded_turn {
        detail::draw_session_detail(f, app, area, theme, selected);
    } else {
        list::draw_session_list(f, app, area, theme);
    }
}

#[cfg(test)]
#[path = "session/tests.rs"]
mod tests;
