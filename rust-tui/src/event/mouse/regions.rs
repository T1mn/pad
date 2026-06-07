use crate::app::App;
use crate::ui;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::event) struct NormalMouseRegions {
    pub(in crate::event) panel_area: Rect,
    pub(in crate::event) panel_inner: Rect,
    pub(in crate::event) preview_area: Rect,
    pub(in crate::event) preview_inner: Rect,
    pub(in crate::event) preview_info_area: Option<Rect>,
    pub(in crate::event) preview_content_area: Rect,
}

fn inner_rect(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub(in crate::event) fn normal_mouse_regions(
    app: &mut App,
    terminal_area: Rect,
) -> NormalMouseRegions {
    let preferred_left_width = Some(ui::panel_list::preferred_panel_width(app));
    let (_, body_layout) = ui::layout::compute_layout(terminal_area, false, preferred_left_width);
    let panel_area = body_layout[0];
    let preview_area = body_layout[1];
    let panel_inner = inner_rect(panel_area);
    let preview_inner = inner_rect(preview_area);

    let (preview_info_area, preview_content_area) = if app.selected_preview_thread().is_some() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(ui::preview::PREVIEW_INFO_CARD_HEIGHT),
                Constraint::Min(0),
            ])
            .split(preview_inner);
        (Some(split[0]), split[1])
    } else {
        (None, preview_inner)
    };

    NormalMouseRegions {
        panel_area,
        panel_inner,
        preview_area,
        preview_inner,
        preview_info_area,
        preview_content_area,
    }
}
