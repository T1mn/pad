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
mod tests {
    use super::{relative_selection, selected_window};

    #[test]
    fn selected_window_keeps_selection_visible() {
        assert_eq!(selected_window(100, 50, 10), 45..55);
        assert_eq!(relative_selection(50, &(45..55)), Some(5));
    }

    #[test]
    fn selected_window_clamps_edges() {
        assert_eq!(selected_window(100, 1, 10), 0..10);
        assert_eq!(selected_window(100, 98, 10), 90..100);
    }

    #[test]
    fn selected_window_handles_empty_or_tiny_viewports() {
        assert_eq!(selected_window(0, 0, 10), 0..0);
        assert_eq!(selected_window(10, 0, 0), 0..0);
        assert_eq!(selected_window(3, 9, 10), 0..3);
    }
}
