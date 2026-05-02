use super::*;

#[test]
fn mouse_click_on_panel_row_selects_it_and_focuses_panel() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.preview.focus = FocusTarget::Preview;

    let area = Rect::new(0, 0, 100, 30);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let click = left_click(regions.panel_inner.x, regions.panel_inner.y + 1);

    mouse::handle_normal_mouse(&mut app, area, click);

    assert_eq!(app.table_state.selected(), Some(1));
    assert!(app.preview.focus == FocusTarget::Panel);
}
#[test]
fn mouse_click_on_panel_row_accounts_for_scroll_offset() {
    let mut app = App::new();
    for idx in 0..6 {
        app.panels.push(sample_panel(
            &format!("%{}", idx + 1),
            &format!("/tmp/p{}", idx),
        ));
    }
    app.table_state = app.table_state.with_offset(3).with_selected(Some(3));

    let area = Rect::new(0, 0, 100, 30);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let click = left_click(regions.panel_inner.x, regions.panel_inner.y);

    mouse::handle_normal_mouse(&mut app, area, click);

    assert_eq!(app.table_state.selected(), Some(3));
}
#[test]
fn mouse_click_on_second_line_of_thread_row_selects_same_item() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();

    let area = Rect::new(0, 0, 100, 30);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let click = left_click(regions.panel_inner.x, regions.panel_inner.y + 2);

    mouse::handle_normal_mouse(&mut app, area, click);

    assert_eq!(app.table_state.selected(), Some(1));
}
#[test]
fn mouse_click_on_session_turn_selects_then_expands_on_repeat() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.preview.source = PreviewSource::Session;
    app.preview.turns = vec![
        PreviewTurn {
            question: "first".into(),
            answer: Some("one".into()),
        },
        PreviewTurn {
            question: "second".into(),
            answer: Some("two".into()),
        },
    ]
    .into();
    app.preview.view = PreviewView::SessionList;

    let area = Rect::new(0, 0, 100, 30);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let click = left_click(
        regions.preview_content_area.x,
        regions.preview_content_area.y + 4,
    );

    mouse::handle_normal_mouse(&mut app, area, click);
    assert!(app.preview.focus == FocusTarget::Preview);
    assert_eq!(app.preview.selected_turn, Some(1));
    assert_eq!(app.preview.expanded_turn, None);

    mouse::handle_normal_mouse(&mut app, area, click);
    assert_eq!(app.preview.expanded_turn, Some(1));
}
#[test]
fn mouse_click_on_session_gap_does_not_change_selection() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.preview.source = PreviewSource::Session;
    app.preview.turns = vec![
        PreviewTurn {
            question: "first".into(),
            answer: Some("one".into()),
        },
        PreviewTurn {
            question: "second".into(),
            answer: Some("two".into()),
        },
    ]
    .into();
    app.preview.view = PreviewView::SessionList;
    app.preview.selected_turn = Some(0);

    let area = Rect::new(0, 0, 100, 30);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let gap_click = left_click(
        regions.preview_content_area.x,
        regions.preview_content_area.y + 3,
    );

    mouse::handle_normal_mouse(&mut app, area, gap_click);

    assert!(app.preview.focus == FocusTarget::Preview);
    assert_eq!(app.preview.selected_turn, Some(0));
    assert_eq!(app.preview.expanded_turn, None);
}
#[test]
fn mouse_wheel_over_preview_scrolls_and_focuses_preview() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.preview.content = (0..20)
        .map(|idx| format!("line {}", idx))
        .collect::<Vec<_>>()
        .join("\n");

    let area = Rect::new(0, 0, 100, 20);
    let regions = mouse::normal_mouse_regions(&mut app, area);
    let wheel = scroll_down(
        regions.preview_content_area.x,
        regions.preview_content_area.y,
    );

    mouse::handle_normal_mouse(&mut app, area, wheel);

    assert!(app.preview.focus == FocusTarget::Preview);
    assert_eq!(app.preview.scroll, MOUSE_PREVIEW_SCROLL_DELTA as u16);
}
