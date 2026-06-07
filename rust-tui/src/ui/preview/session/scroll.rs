use crate::app::App;

pub(crate) fn resolve_session_list_scroll(
    app: &mut App,
    selected_range: Option<(usize, usize)>,
    viewport_height: u16,
    total_lines: usize,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let mut scroll = app
        .preview
        .list_scroll
        .min(max_scroll.min(u16::MAX as usize) as u16);

    if app.preview.follow_selection {
        if let Some((start, end)) = selected_range {
            let scroll_usize = scroll as usize;
            let viewport = viewport_height as usize;
            if start < scroll_usize {
                scroll = start.min(max_scroll).min(u16::MAX as usize) as u16;
            } else if end >= scroll_usize.saturating_add(viewport) {
                let adjusted = end
                    .saturating_add(1)
                    .saturating_sub(viewport)
                    .min(max_scroll)
                    .min(u16::MAX as usize);
                scroll = adjusted as u16;
            }
        }
    }

    app.preview.list_scroll = scroll;
    scroll
}

pub(crate) fn resolve_preview_scroll_for_line_count(
    app: &mut App,
    total_lines: usize,
    viewport_height: u16,
) -> u16 {
    if viewport_height == 0 {
        return 0;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height as usize);
    let max_scroll = max_scroll.min(u16::MAX as usize) as u16;
    let scroll = if app.preview_uses_detail_scroll() {
        app.preview.detail_scroll.min(max_scroll)
    } else if app.preview.follow_bottom {
        max_scroll
    } else {
        app.preview.scroll.min(max_scroll)
    };
    if app.preview_uses_detail_scroll() {
        app.preview.detail_scroll = scroll;
    } else {
        app.preview.scroll = scroll;
    }
    scroll
}

pub(crate) fn visible_detail_window(
    total_lines: usize,
    scroll: u16,
    viewport_height: u16,
) -> std::ops::Range<usize> {
    let start = scroll as usize;
    let end = start
        .saturating_add(viewport_height as usize)
        .min(total_lines);
    start..end
}
