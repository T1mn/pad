use super::compute_layout;
use ratatui::layout::Rect;

#[test]
fn normal_layout_allows_wider_agents_panel_on_large_terminals() {
    let area = Rect::new(0, 0, 140, 40);
    let (_main, body) = compute_layout(area, false, Some(46));

    assert_eq!(body[0].width, 38);
    assert_eq!(body[1].width, 102);
}
