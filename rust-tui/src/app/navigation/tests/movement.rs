use super::*;

#[test]
fn next_uses_folder_rows_when_not_expanded() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sync_sidebar_selection();

    app.next();

    assert_eq!(app.table_state.selected(), Some(1));
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/beta")
    );
}

#[test]
fn next_skips_expanded_folder_row() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    app.next();

    assert_eq!(app.table_state.selected(), Some(1));
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
}

#[test]
fn next_skips_search_expanded_folder_row() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.search_query = "alpha".into();
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    app.next();

    assert_eq!(app.table_state.selected(), Some(1));
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
}

#[test]
fn numeric_jump_ignores_folder_rows_and_hidden_threads() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    app.jump_to(0);
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));

    app.jump_to(1);
    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%1"));
}

#[test]
fn numeric_jump_uses_visible_thread_order() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.sidebar.expanded_folders.insert("/tmp/alpha".into());
    app.sidebar.expanded_folders.insert("/tmp/beta".into());
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    app.jump_to(1);

    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%2"));
    assert_eq!(app.table_state.selected(), Some(3));
}

#[test]
fn numeric_jump_uses_filtered_visible_threads() {
    let mut app = App::new();
    app.panels.push(sample_panel("%1", "/tmp/alpha"));
    app.panels.push(sample_panel("%2", "/tmp/beta"));
    app.search_query = "beta".into();
    app.invalidate_sidebar_visible_cache();
    app.sync_sidebar_selection();

    app.jump_to(0);

    assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%2"));
}

#[test]
fn shift_j_k_moves_selected_thread_without_following_completion_sort() {
    let mut app = App::new();
    let mut first = sample_panel("%1", "/tmp/alpha");
    first.agent_session_id = Some("sid-1".into());
    let mut second = sample_panel("%2", "/tmp/beta");
    second.agent_session_id = Some("sid-2".into());
    app.panels.push(first);
    app.panels.push(second);
    app.sync_sidebar_selection();

    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/alpha", "/tmp/beta"]);
    assert!(app.move_selected_sidebar_item_down());
    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/beta", "/tmp/alpha"]);
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );

    assert!(app.move_selected_sidebar_item_up());
    assert_eq!(visible_item_keys(&mut app), vec!["/tmp/alpha", "/tmp/beta"]);
    assert_eq!(
        app.sidebar.selected_sidebar_key.as_deref(),
        Some("/tmp/alpha")
    );
}
