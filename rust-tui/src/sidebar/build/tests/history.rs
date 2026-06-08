use super::support::{cached_snapshot, codex_thread, folder};

#[test]
fn codex_history_prefers_session_cache_prompt_for_subtitle() {
    let snapshot = cached_snapshot("newest prompt", Some("answer"));
    let thread = build_codex_history_entry(&folder(), &codex_thread(), Some(&snapshot), false);

    assert_eq!(thread.subtitle.as_deref(), Some("newest prompt"));
    assert_eq!(thread.last_user_prompt.as_deref(), Some("newest prompt"));
    assert_eq!(thread.cached_preview_turns.len(), 1);
}

#[test]
fn active_view_history_entries_do_not_sort_by_updated_at_without_explicit_activity() {
    let thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    assert_eq!(thread.updated_at, 42);
    assert_eq!(thread.sort_updated_at, 0);
}

#[test]
fn archived_view_history_entries_keep_updated_at_sorting() {
    let thread = build_codex_history_entry(&folder(), &codex_thread(), None, true);
    assert_eq!(thread.updated_at, 42);
    assert_eq!(thread.sort_updated_at, 42);
}
