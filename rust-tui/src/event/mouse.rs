use crate::app::App;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

mod click {
    use crate::app::App;
    use crate::{model::PreviewView, ui};
    use ratatui::layout::Rect;

    pub(super) fn handle_normal_left_click(
        app: &mut App,
        terminal_area: Rect,
        column: u16,
        row: u16,
    ) {
        let regions = super::normal_mouse_regions(app, terminal_area);

        if super::hit_test::rect_contains(regions.panel_area, column, row) {
            click_panel(app, regions.panel_inner, column, row);
            app.focus_panel();
            return;
        }

        if !super::hit_test::rect_contains(regions.preview_area, column, row)
            || !app.focus_preview()
        {
            return;
        }

        if click_preview_info(app, regions.preview_info_area, column, row) {
            return;
        }

        if click_preview_turn(app, regions.preview_content_area, column, row) {
            return;
        }

        if super::hit_test::rect_contains(regions.preview_content_area, column, row)
            && preview_mouse_copy_enabled(app)
        {
            app.begin_preview_mouse_selection(column, row);
        }
    }

    fn click_panel(app: &mut App, panel_inner: Rect, column: u16, row: u16) {
        if !super::hit_test::rect_contains(panel_inner, column, row) {
            return;
        }

        let table_offset = app.table_state.offset();
        let items = app.visible_sidebar_items_ref();
        let Some(index) =
            super::hit_test::panel_index_at_position(panel_inner, row, table_offset, items)
        else {
            return;
        };

        let is_folder = items
            .get(index)
            .is_some_and(|item| item.as_folder().is_some());
        if is_folder {
            let _ = app.select_sidebar_index(index, false);
            let _ = app.toggle_selected_folder();
        } else {
            let _ = app.jump_to_sidebar_index(index);
        }
    }

    fn click_preview_info(app: &mut App, info_area: Option<Rect>, column: u16, row: u16) -> bool {
        let Some(info_area) = info_area else {
            return false;
        };
        if !super::hit_test::rect_contains(info_area, column, row) {
            return false;
        }

        if let Some(session_id) = ui::preview::preview_sid_text_at(app, info_area, column, row) {
            let _ = app.copy_text_with_toast("SID", &session_id);
        } else if let Some(share_url) =
            ui::preview::preview_share_url_text_at(app, info_area, column, row)
        {
            let _ = app.copy_text_with_toast("SHARE", &share_url);
        }
        true
    }

    fn click_preview_turn(
        app: &mut App,
        preview_content_area: Rect,
        column: u16,
        row: u16,
    ) -> bool {
        if !app.has_session_preview_turns()
            || app.preview.view != PreviewView::SessionList
            || !super::hit_test::rect_contains(preview_content_area, column, row)
        {
            return false;
        }

        if let Some(index) = super::hit_test::session_turn_index_at_position(
            preview_content_area,
            row,
            app.preview.list_scroll,
            app.preview.turns.len(),
        ) {
            if app.preview.selected_turn == Some(index) {
                let _ = app.toggle_preview_turn_expanded();
            } else {
                let _ = app.select_preview_turn(index);
            }
        }
        true
    }

    fn preview_mouse_copy_enabled(app: &App) -> bool {
        !(app.has_session_preview_turns() && app.preview.view == PreviewView::SessionList)
    }
}
mod hit_test {
    use ratatui::layout::Rect;

    pub(super) fn rect_contains(area: Rect, column: u16, row: u16) -> bool {
        area.width > 0
            && area.height > 0
            && column >= area.x
            && column < area.x.saturating_add(area.width)
            && row >= area.y
            && row < area.y.saturating_add(area.height)
    }

    pub(super) fn panel_index_at_position(
        panel_inner: Rect,
        row: u16,
        table_offset: usize,
        items: &[crate::sidebar::SidebarItem],
    ) -> Option<usize> {
        if items.is_empty() || !rect_contains(panel_inner, panel_inner.x, row) {
            return None;
        }

        let mut remaining = row.saturating_sub(panel_inner.y) as usize;
        for (index, item) in items.iter().enumerate().skip(table_offset) {
            let height = crate::ui::panel_list::item_row_height(item);
            if remaining < height {
                return Some(index);
            }
            remaining = remaining.saturating_sub(height);
        }

        None
    }
    pub(super) fn session_turn_index_at_position(
        preview_content_area: Rect,
        row: u16,
        scroll: u16,
        turn_count: usize,
    ) -> Option<usize> {
        if turn_count == 0 || !rect_contains(preview_content_area, preview_content_area.x, row) {
            return None;
        }

        let line = scroll as usize + row.saturating_sub(preview_content_area.y) as usize;
        crate::ui::preview::session_turn_index_at_line(line, turn_count)
    }
}
mod hover {
    use crate::app::App;
    use ratatui::layout::Rect;

    pub(super) fn update_hovered_folder(app: &mut App, terminal_area: Rect, column: u16, row: u16) {
        if app.should_defer_ui_updates() || app.frame_budget_exceeded {
            return;
        }

        let regions = super::normal_mouse_regions(app, terminal_area);
        let hovered_folder_key = if super::hit_test::rect_contains(regions.panel_inner, column, row)
        {
            let table_offset = app.table_state.offset();
            let items = app.visible_sidebar_items_ref();
            super::hit_test::panel_index_at_position(regions.panel_inner, row, table_offset, items)
                .and_then(|index| items.get(index).cloned())
                .and_then(|item| item.as_folder().map(|folder| folder.key.clone()))
        } else {
            None
        };

        if hovered_folder_key != app.sidebar.hovered_folder_key {
            app.sidebar.hovered_folder_key = hovered_folder_key;
            app.dirty = true;
        }
    }
}
mod regions {
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
        let (_, body_layout) =
            ui::layout::compute_layout(terminal_area, false, preferred_left_width);
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
}
mod scroll {
    use crate::app::App;
    use crossterm::event::{self, Event, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;
    use std::io;
    use std::time::Duration;

    pub(in crate::event) const MOUSE_PREVIEW_SCROLL_DELTA: i32 = 3;

    pub(in crate::event) fn handle_normal_scroll(
        app: &mut App,
        terminal_area: Rect,
        column: u16,
        row: u16,
        delta: i32,
    ) {
        let regions = super::normal_mouse_regions(app, terminal_area);

        if super::hit_test::rect_contains(regions.panel_area, column, row) {
            app.focus_panel();
            if !app.visible_sidebar_items_ref().is_empty() {
                if delta < 0 {
                    app.previous();
                } else {
                    app.next();
                }
            }
            return;
        }

        if super::hit_test::rect_contains(regions.preview_area, column, row) && app.focus_preview()
        {
            app.scroll_preview_by(delta * MOUSE_PREVIEW_SCROLL_DELTA);
        }
    }

    pub(in crate::event) fn mouse_scroll_delta(kind: &MouseEventKind) -> Option<i32> {
        match kind {
            MouseEventKind::ScrollUp => Some(-1),
            MouseEventKind::ScrollDown => Some(1),
            _ => None,
        }
    }

    pub(in crate::event) fn coalesce_scroll_burst(
        first: MouseEvent,
        carried_event: &mut Option<Event>,
    ) -> io::Result<(u16, u16, i32)> {
        let mut column = first.column;
        let mut row = first.row;
        let mut delta = mouse_scroll_delta(&first.kind).unwrap_or_default();

        while event::poll(Duration::from_millis(0))? {
            let next = event::read()?;
            match next {
                Event::Mouse(mouse) if mouse_scroll_delta(&mouse.kind).is_some() => {
                    column = mouse.column;
                    row = mouse.row;
                    delta += mouse_scroll_delta(&mouse.kind).unwrap_or_default();
                }
                other => {
                    *carried_event = Some(other);
                    break;
                }
            }
        }

        Ok((column, row, delta))
    }

    pub(in crate::event) fn drain_pending_scroll_events(
        carried_event: &mut Option<Event>,
    ) -> io::Result<usize> {
        let mut dropped = 0usize;
        while event::poll(Duration::from_millis(0))? {
            let next = event::read()?;
            match next {
                Event::Mouse(mouse) if mouse_scroll_delta(&mouse.kind).is_some() => {
                    dropped += 1;
                }
                other => {
                    *carried_event = Some(other);
                    break;
                }
            }
        }
        Ok(dropped)
    }
}
mod selection {
    use crate::app::App;
    use crate::ui;
    use ratatui::layout::Rect;

    pub(super) fn update_preview_mouse_selection(
        app: &mut App,
        terminal_area: Rect,
        column: u16,
        row: u16,
    ) {
        let regions = super::normal_mouse_regions(app, terminal_area);
        let (column, row) = clamp_to_preview_content(regions.preview_content_area, column, row);
        let _ = app.update_preview_mouse_selection(column, row);
    }

    pub(super) fn finish_preview_mouse_selection(
        app: &mut App,
        terminal_area: Rect,
        column: u16,
        row: u16,
    ) {
        let Some(selection) = app.finish_preview_mouse_selection() else {
            return;
        };

        let regions = super::normal_mouse_regions(app, terminal_area);
        if let Some(text) = ui::preview::extract_preview_selection_text(
            app,
            regions.preview_content_area,
            (selection.anchor_column, selection.anchor_row),
            (column, row),
        ) {
            let _ = app.copy_text_with_toast("内容", &text);
        }
    }

    fn clamp_to_preview_content(area: Rect, column: u16, row: u16) -> (u16, u16) {
        (
            column.clamp(area.x, area.right().saturating_sub(1)),
            row.clamp(area.y, area.bottom().saturating_sub(1)),
        )
    }
}

pub(in crate::event) use regions::normal_mouse_regions;
#[cfg(test)]
pub(in crate::event) use scroll::MOUSE_PREVIEW_SCROLL_DELTA;
pub(in crate::event) use scroll::{
    coalesce_scroll_burst, drain_pending_scroll_events, handle_normal_scroll, mouse_scroll_delta,
};

pub(super) fn handle_normal_mouse(app: &mut App, terminal_area: Rect, mouse: MouseEvent) {
    if app.sidebar.show_tree {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            click::handle_normal_left_click(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Drag(MouseButton::Left) if app.preview.mouse_selection().is_some() => {
            selection::update_preview_mouse_selection(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            selection::finish_preview_mouse_selection(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::Moved => {
            if app.preview.mouse_selection().is_some() {
                selection::update_preview_mouse_selection(
                    app,
                    terminal_area,
                    mouse.column,
                    mouse.row,
                );
                return;
            }
            hover::update_hovered_folder(app, terminal_area, mouse.column, mouse.row);
        }
        MouseEventKind::ScrollUp => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            scroll::handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, -1);
        }
        MouseEventKind::ScrollDown => {
            app.clear_panel_tab();
            let _ = app.clear_preview_mouse_selection();
            scroll::handle_normal_scroll(app, terminal_area, mouse.column, mouse.row, 1);
        }
        _ => {}
    }
}
