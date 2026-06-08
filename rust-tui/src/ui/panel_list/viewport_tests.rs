use super::render_window;

#[test]
fn keeps_selected_near_middle_when_possible() {
    let range = render_window(20, Some(10), 5, |_| 1);

    assert_eq!(range, 8..13);
}

#[test]
fn fills_from_top_when_selection_is_near_start() {
    let range = render_window(20, Some(1), 5, |_| 1);

    assert_eq!(range, 0..5);
}

#[test]
fn respects_tall_thread_rows() {
    let range = render_window(20, Some(5), 6, |idx| if idx % 2 == 0 { 1 } else { 2 });

    assert!(range.contains(&5));
    let total_height: usize = range
        .clone()
        .map(|idx| if idx % 2 == 0 { 1 } else { 2 })
        .sum();
    assert!(total_height <= 6);
}
