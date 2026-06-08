use super::*;

#[test]
fn help_page_callbacks_parse() {
    assert_eq!(
        HelpPage::from_callback("help:overview"),
        Some(HelpPage::Overview)
    );
    assert_eq!(HelpPage::from_callback("help:codex"), Some(HelpPage::Codex));
    assert_eq!(
        HelpPage::from_callback("help:workflow"),
        Some(HelpPage::Workflow)
    );
    assert_eq!(HelpPage::from_callback("help:list"), None);
}
#[test]
fn help_page_html_includes_target_and_commands() {
    let state = TelegramState {
        selected_target: Some(SelectedTarget {
            pane_id: "%7".into(),
            label: "X rust-tui".into(),
        }),
        ..TelegramState::default()
    };
    let codex_html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Codex);
    assert!(codex_html.contains("Pad Telegram"));
    assert!(codex_html.contains("X rust-tui"));
    assert!(codex_html.contains("/status"));
    assert!(codex_html.contains("/compact"));

    let overview_html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Overview);
    assert!(overview_html.contains("/history"));
    assert!(overview_html.contains("/diag"));
    assert!(overview_html.contains("/restart"));
    assert!(overview_html.contains("/reset"));
}

#[test]
fn help_page_html_escapes_target_label() {
    let state = TelegramState {
        selected_target: Some(SelectedTarget {
            pane_id: "%7".into(),
            label: "A&B <codex> 東".into(),
        }),
        ..TelegramState::default()
    };

    let html = help_page_html(crate::i18n::Locale::En, &state, HelpPage::Overview);
    assert!(html.contains("A&amp;B &lt;codex&gt; 東"));
    assert!(!html.contains("A&B <codex> 東"));
}

#[test]
fn help_keyboard_marks_active_page() {
    let keyboard = build_help_keyboard(crate::i18n::Locale::En, HelpPage::Workflow);
    assert_eq!(keyboard.len(), 2);
    assert_eq!(keyboard[0][2]["callback_data"], "help:workflow");
    assert_eq!(keyboard[1][0]["callback_data"], "help:list");
}
