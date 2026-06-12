use super::labels::{display_scope_title_label, special_view_title_label};
use super::metrics;
use crate::app::state::{PreferredPanelWidthCache, ThreadListView};
use crate::app::App;
use crate::sidebar::SidebarItem;

const MIN_PANEL_WIDTH: u16 = 6;
const MAX_PANEL_WIDTH: u16 = 90;
const FOLDER_LABEL_WIDTH_LIMIT: usize = 40;
const THREAD_TITLE_WIDTH_LIMIT: usize = 72;

pub fn preferred_panel_width(app: &mut App) -> u16 {
    let thread_list_view = app.thread_list_view();
    let locale = app.locale;
    let live_only = app.showing_live_sessions();
    let manual_width = app.config.display.agent_panel_width;
    if !app.sidebar.visible_sidebar_items_dirty {
        if let Some(cache) = app.sidebar.preferred_panel_width_cache {
            if cache.locale == locale
                && cache.thread_list_view == thread_list_view
                && cache.live_only == live_only
                && cache.manual_width == manual_width
            {
                return cache.width;
            }
        }
    }

    let title_width = if thread_list_view != ThreadListView::Normal {
        metrics::display_width(&format!(
            " {} {} {} ",
            "○",
            special_view_title_label(locale, thread_list_view),
            88
        ))
    } else {
        metrics::display_width(&format!(
            " {} {} {} ",
            "○",
            display_scope_title_label(locale, live_only),
            888
        ))
    };
    let items = app.visible_sidebar_items_ref();
    let mut content_width = 10usize;
    for item in items {
        let item_width = match item {
            SidebarItem::Folder(folder) => {
                2 + metrics::display_width(&metrics::truncate_to_width(
                    &folder.label,
                    FOLDER_LABEL_WIDTH_LIMIT,
                ))
            }
            SidebarItem::Thread(thread) => thread_item_width(&thread.title),
        };
        content_width = content_width.max(item_width);
        if content_width >= MAX_PANEL_WIDTH as usize {
            break;
        }
    }
    let auto_width =
        (content_width.max(title_width) as u16 + 6).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
    let width = manual_width
        .map(|manual| auto_width.max(manual.clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH)))
        .unwrap_or(auto_width);
    app.sidebar.preferred_panel_width_cache = Some(PreferredPanelWidthCache {
        width,
        locale,
        thread_list_view,
        live_only,
        manual_width,
    });
    width
}

fn thread_item_width(title: &str) -> usize {
    let title_width =
        metrics::display_width(&metrics::truncate_to_width(title, THREAD_TITLE_WIDTH_LIMIT));
    9 + title_width
}

#[cfg(test)]
#[path = "width_tests.rs"]
mod tests;
