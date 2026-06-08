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
