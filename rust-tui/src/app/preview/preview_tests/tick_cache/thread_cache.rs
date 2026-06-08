#[test]
fn thread_preview_cache_prunes_to_max_entries() {
    let mut app = App::new();
    let base_ts = 1_000_000i64;
    let total = THREAD_PREVIEW_CACHE_MAX_ENTRIES + 8;
    for i in 0..total {
        let ts = base_ts + i as i64;
        app.preview.thread_preview_cache.insert(
            format!("thread:{}", i),
            ThreadPreviewCacheEntry {
                turns: Default::default(),
                session_cache_state: None,
                transcript_path: None,
                session_id: None,
                updated_at: Some(ts),
                cached_at: ts,
            },
        );
    }

    assert!(app.prune_thread_preview_cache());
    assert_eq!(
        app.preview.thread_preview_cache.len(),
        THREAD_PREVIEW_CACHE_MAX_ENTRIES
    );
    assert!(app
        .preview
        .thread_preview_cache
        .contains_key(&format!("thread:{}", total - 1)));
    assert!(!app.preview.thread_preview_cache.contains_key("thread:0"));
}
