use super::compute_layout;
use ratatui::layout::Rect;

pub(crate) fn normal_layout_allows_wider_agents_panel_on_large_terminals() {
    let area = Rect::new(0, 0, 140, 40);
    let (_main, body) = compute_layout(area, false, Some(84));

    assert_eq!(body[0].width, 84);
    assert_eq!(body[1].width, 56);
}

pub(crate) fn normal_layout_keeps_preview_space_on_medium_terminals() {
    let area = Rect::new(0, 0, 80, 30);
    let (_main, body) = compute_layout(area, false, Some(84));

    assert_eq!(body[0].width, 44);
    assert_eq!(body[1].width, 36);

    let collapsed_area = Rect::new(0, 0, 100, 30);
    let (_main, normal_body) = compute_layout(collapsed_area, false, Some(0));
    let (_main, tree_body) = compute_layout(collapsed_area, true, Some(0));

    assert_eq!(normal_body[0].width, 0);
    assert_eq!(normal_body[1].width, 100);
    assert_eq!(tree_body[0].width, 0);
    assert_eq!(tree_body[1].width, 100);
}
