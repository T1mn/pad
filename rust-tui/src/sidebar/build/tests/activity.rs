use super::support::{cached_snapshot, codex_thread, folder, live_codex_thread_without_prompt};

#[test]
fn merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one() {
    let mut threads = vec![live_codex_thread_without_prompt()];
    let snapshot = cached_snapshot("newest prompt", None);
    let history = build_codex_history_entry(&folder(), &codex_thread(), Some(&snapshot), false);

    merge_or_insert_thread(&mut threads, history, &[], &HashMap::new());

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].subtitle.as_deref(), Some("newest prompt"));
    assert_eq!(
        threads[0].last_user_prompt.as_deref(),
        Some("newest prompt")
    );
    assert_eq!(threads[0].cached_preview_turns.len(), 1);
    assert_eq!(
        threads[0].session_cache_state,
        Some(SessionCacheState::Cached)
    );
}

#[test]
fn runtime_sort_activity_updates_history_order() {
    let mut threads = Vec::new();
    let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    let runtime = HashMap::from([(String::from("codex:path:/repo/.codex/sid-1.jsonl"), 120)]);

    merge_or_insert_thread(&mut threads, history, &[], &runtime);

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].sort_updated_at, 120);
}
