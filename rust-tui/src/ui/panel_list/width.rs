use super::labels::{display_scope_title_label, special_view_title_label};
use super::{metrics, thread_subtitle};
use crate::app::state::{PreferredPanelWidthCache, ThreadListView};
use crate::app::App;
use crate::sidebar::SidebarItem;

pub fn preferred_panel_width(app: &mut App) -> u16 {
    let thread_list_view = app.thread_list_view();
    let locale = app.locale;
    let live_only = app.showing_live_sessions();
    if !app.sidebar.visible_sidebar_items_dirty {
        if let Some(cache) = app.sidebar.preferred_panel_width_cache {
            if cache.locale == locale
                && cache.thread_list_view == thread_list_view
                && cache.live_only == live_only
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
                2 + metrics::display_width(&metrics::truncate_to_width(&folder.label, 28))
            }
            SidebarItem::Thread(thread) => {
                let subtitle = thread_subtitle::thread_subtitle(thread);
                let title_width =
                    metrics::display_width(&metrics::truncate_to_width(&thread.title, 38));
                let subtitle_width =
                    metrics::display_width(&metrics::truncate_to_width(&subtitle, 32));
                9 + title_width.max(subtitle_width)
            }
        };
        content_width = content_width.max(item_width);
        if content_width >= 40 {
            break;
        }
    }
    let width = (content_width.max(title_width) as u16 + 6).clamp(6, 46);
    app.sidebar.preferred_panel_width_cache = Some(PreferredPanelWidthCache {
        width,
        locale,
        thread_list_view,
        live_only,
    });
    width
}

#[cfg(test)]
#[path = "width_tests.rs"]
mod tests;
