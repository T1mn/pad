use super::thread_action_confirm_body;

#[test]
fn thread_action_body_keeps_blank_lines_without_warning() {
    assert_eq!(
        thread_action_confirm_body(
            "Archive this thread?",
            "Demo thread",
            None,
            "Enter: confirm",
            "Esc: cancel",
        ),
        "Archive this thread?\n\nDemo thread\n\nEnter: confirm\nEsc: cancel"
    );
}

#[test]
fn thread_action_body_includes_warning_block() {
    assert_eq!(
        thread_action_confirm_body(
            "Archive this thread?",
            "Demo thread",
            Some("Live pane is still running"),
            "Enter: confirm",
            "Esc: cancel",
        ),
        "Archive this thread?\n\nDemo thread\n\nLive pane is still running\n\nEnter: confirm\nEsc: cancel"
    );
}
