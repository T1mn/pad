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
