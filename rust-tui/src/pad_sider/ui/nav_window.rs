use std::ops::Range;

pub(super) fn selected_window(len: usize, selected: usize, viewport_height: usize) -> Range<usize> {
    if len == 0 || viewport_height == 0 {
        return 0..0;
    }

    let height = viewport_height.min(len);
    let selected = selected.min(len - 1);
    let mut start = selected.saturating_sub(height / 2);
    if start + height > len {
        start = len - height;
    }
    start..start + height
}

pub(super) fn relative_selection(selected: usize, range: &Range<usize>) -> Option<usize> {
    range
        .contains(&selected)
        .then_some(selected.saturating_sub(range.start))
}

pub(super) fn list_viewport_height(area_height: u16) -> usize {
    area_height.saturating_sub(2) as usize
}

#[cfg(test)]
#[path = "nav_window_tests.rs"]
mod tests;
