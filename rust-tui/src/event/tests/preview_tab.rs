use super::*;

#[test]
fn single_tab_from_detail_keeps_current_behavior_and_focuses_panel() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("live:%1".into());
    app.preview.turns = vec![PreviewTurn {
        question: "first".into(),
        answer: Some("one".into()),
    }]
    .into();
    app.preview.view = PreviewView::SessionDetail;
    app.preview.selected_turn = Some(0);
    app.preview.expanded_turn = Some(0);
    app.preview.focus = FocusTarget::Preview;
    app.sync_sidebar_selection();

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();

    assert!(app.preview.focus == FocusTarget::Panel);
    assert_eq!(app.preview.view, PreviewView::SessionDetail);
}
#[test]
fn double_tab_from_detail_restores_session_list_and_keeps_panel_focus() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.preview.source = PreviewSource::Session;
    app.preview.pane_id = Some("live:%1".into());
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
    app.preview.view = PreviewView::SessionDetail;
    app.preview.selected_turn = Some(1);
    app.preview.expanded_turn = Some(1);
    app.preview.focus = FocusTarget::Preview;
    app.sync_sidebar_selection();

    let mut terminal = test_terminal();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();
    handle_normal_mode(&mut terminal, &mut app, key(KeyCode::Tab)).unwrap();

    assert!(app.preview.focus == FocusTarget::Panel);
    assert_eq!(app.preview.view, PreviewView::SessionList);
    assert_eq!(app.preview.selected_turn, Some(1));
    assert_eq!(app.preview.expanded_turn, None);
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );
}
