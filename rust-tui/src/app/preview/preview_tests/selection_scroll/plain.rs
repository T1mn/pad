#[test]
fn preview_update_plain_view_follow_bottom_depends_on_target_change() {
    struct Case {
        name: &'static str,
        previous_pane: Option<&'static str>,
        target: &'static str,
        initial_follow_bottom: bool,
        expected_follow_bottom: bool,
    }

    let cases = vec![
        Case {
            name: "same target keeps false",
            previous_pane: Some("%1"),
            target: "%1",
            initial_follow_bottom: false,
            expected_follow_bottom: false,
        },
        Case {
            name: "target switch forces true",
            previous_pane: Some("%1"),
            target: "%2",
            initial_follow_bottom: false,
            expected_follow_bottom: true,
        },
        Case {
            name: "existing true stays true",
            previous_pane: Some("%1"),
            target: "%1",
            initial_follow_bottom: true,
            expected_follow_bottom: true,
        },
        Case {
            name: "missing previous target defaults true",
            previous_pane: None,
            target: "%1",
            initial_follow_bottom: false,
            expected_follow_bottom: true,
        },
    ];

    for case in cases {
        let mut app = App::new();
        app.preview.pane_id = case.previous_pane.map(|pane| pane.to_string());
        app.preview.source = PreviewSource::Tmux;
        app.preview.view = PreviewView::Plain;
        app.preview.content = "before".into();
        app.preview.follow_bottom = case.initial_follow_bottom;
        app.preview.follow_selection = false;
        app.dirty = false;

        send_preview_update(
            &mut app,
            PreviewUpdate {
                target_key: case.target.into(),
                live_pane_id: Some(case.target.into()),
                content: "after".into(),
                source: PreviewSource::Tmux,
                session_origin: None,
                session_id: None,
                turns: Default::default(),
                transcript_path: None,
                session_cache_state: None,
                updated_at: None,
            },
        );

        assert_eq!(
            app.preview.follow_bottom, case.expected_follow_bottom,
            "{}",
            case.name
        );
        assert_eq!(app.preview.view, PreviewView::Plain, "{}", case.name);
        assert!(app.preview.turns.is_empty(), "{}", case.name);
        assert!(app.preview.follow_selection, "{}", case.name);
    }
}
