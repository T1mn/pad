use super::super::{folder_row, thread_row};
use crate::sidebar::SidebarItem;
use ratatui::widgets::Row;

pub(super) fn build_sidebar_row(
    item: &SidebarItem,
    jump_badge: Option<usize>,
    is_selected: bool,
    content_width: usize,
    theme: &crate::theme::Theme,
    is_expanded: bool,
    is_hovered_folder: bool,
) -> Row<'static> {
    match item {
        SidebarItem::Folder(folder) => folder_row::build_folder_row(
            folder,
            is_selected,
            content_width,
            theme,
            is_expanded,
            is_hovered_folder,
        ),
        SidebarItem::Thread(thread) => {
            thread_row::build_thread_row(thread, is_selected, content_width, theme, jump_badge)
        }
    }
}
