use super::support::{codex_thread, folder, generated_title_meta};

#[test]
fn manual_title_override_wins_over_generated_summary_for_title() {
    let mut thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    apply_thread_meta(&mut thread, &generated_title_meta(Some("Manual title")));

    assert_eq!(thread.title, "Manual title");
    assert_eq!(thread.generated_title.as_deref(), Some("Generated title"));
}

#[test]
fn generated_summary_does_not_replace_session_title() {
    let mut thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    apply_thread_meta(&mut thread, &generated_title_meta(None));

    assert_eq!(thread.title, "upstream title");
    assert_eq!(thread.generated_title.as_deref(), Some("Generated title"));
}
