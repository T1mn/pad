use super::support::{cached_snapshot, codex_thread, folder, live_codex_thread_without_prompt};

#[test]
fn merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one() {
    let mut threads = vec![live_codex_thread_without_prompt()];
    let snapshot = cached_snapshot("newest prompt", None);
    let history = build_codex_history_entry(&folder(), &codex_thread(), Some(&snapshot), false);

    merge_or_insert_thread(&mut threads, history, &[], &HashMap::new(), &HashMap::new());

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
fn startup_sort_seed_applies_when_runtime_activity_is_missing() {
    let mut threads = Vec::new();
    let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    let startup = HashMap::from([(String::from("codex:sid:sid-1"), 99)]);

    merge_or_insert_thread(&mut threads, history, &[], &HashMap::new(), &startup);

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].sort_updated_at, 99);
}

#[test]
fn runtime_sort_activity_overrides_startup_seed() {
    let mut threads = Vec::new();
    let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
    let runtime = HashMap::from([(String::from("codex:path:/repo/.codex/sid-1.jsonl"), 120)]);
    let startup = HashMap::from([(String::from("codex:sid:sid-1"), 99)]);

    merge_or_insert_thread(&mut threads, history, &[], &runtime, &startup);

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].sort_updated_at, 120);
}
