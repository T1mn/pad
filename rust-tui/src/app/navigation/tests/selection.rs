use super::*;

#[test]
fn selected_preview_thread_resolves_from_folder_summary_selection() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.sync_sidebar_selection();

    let thread = app
        .selected_preview_thread()
        .expect("folder selection resolves");

    assert_eq!(thread.key, "live:%1");
    assert_eq!(thread.live_pane_id.as_deref(), Some("%1"));
}

#[test]
fn sync_sidebar_selection_recovers_collapsed_thread_to_folder_key() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sidebar.selected_sidebar_key = Some("live:%1".into());
    app.sync_sidebar_selection();

    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

    app.sidebar.expanded_folders.clear();
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );
    assert_eq!(app.table_state.selected(), Some(0));
}

#[test]
fn sync_sidebar_selection_falls_back_to_first_visible_item_when_selected_key_is_filtered_out() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sidebar.selected_sidebar_key = Some("live:%1".into());
    app.sync_sidebar_selection();
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

    app.search_query = "beta".into();
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/beta", "live:%2"]);
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/beta")
    );
    assert_eq!(app.table_state.selected(), Some(0));
}

#[test]
fn visible_sidebar_items_sequence_stays_stable_across_expand_and_search() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/alpha"));
    app.panels.push(sample_panel("%3", "/tmp/beta"));

    app.sync_sidebar_selection();
    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/alpha", "/tmp/beta"]);

    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    assert_eq!(
        visible_item_keys(&mut app),
        vec!["/tmp/alpha", "live:%1", "live:%2", "/tmp/beta"]
    );

    app.search_query = "beta".into();
    app.invalidate_sidebar_visible_cache();
    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/beta", "live:%3"]);
}
