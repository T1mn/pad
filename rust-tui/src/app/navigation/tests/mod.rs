use crate::app::App;
use crate::model::{AgentPanel, AgentState, AgentType};

pub(super) fn visible_item_keys(app: &mut App) -> Vec<String> {
    app.visible_sidebar_items_ref()
        .iter()
        .map(|item| item.key().to_string())
        .collect()
}

pub(super) fn sample_panel(pane_id: &str, working_dir: &str) -> AgentPanel {
    AgentPanel {
        session: "0".into(),
        window: "main".into(),
        window_index: "1".into(),
        pane: "1".into(),
        pane_id: pane_id.into(),
        agent_type: AgentType::Codex,
        working_dir: working_dir.into(),
        is_active: true,
        state: AgentState::Idle,
        transcript_path: None,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        agent_session_id: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
    }
}

pub(crate) mod movement {
    use super::*;

    pub(crate) fn next_uses_folder_rows_when_not_expanded() {
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

    pub(crate) fn next_skips_expanded_folder_row() {
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

    pub(crate) fn next_skips_search_expanded_folder_row() {
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

    pub(crate) fn numeric_jump_ignores_folder_rows_and_hidden_threads() {
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

    pub(crate) fn numeric_jump_uses_visible_thread_order() {
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

    pub(crate) fn numeric_jump_uses_filtered_visible_threads() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.panels.push(sample_panel("%2", "/tmp/beta"));
        app.search_query = "beta".into();
        app.invalidate_sidebar_visible_cache();
        app.sync_sidebar_selection();

        app.jump_to(0);

        assert_eq!(app.sidebar.selected_sidebar_key.as_deref(), Some("live:%2"));
    }

    pub(crate) fn shift_j_k_moves_selected_thread_without_following_completion_sort() {
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
}
pub(crate) mod selection {
    use super::*;

    pub(crate) fn selected_preview_thread_resolves_from_folder_summary_selection() {
        let mut app = App::new();
        app.panels.push(sample_panel("%1", "/tmp/alpha"));
        app.sync_sidebar_selection();

        let thread = app
            .selected_preview_thread()
            .expect("folder selection resolves");

        assert_eq!(thread.key, "live:%1");
        assert_eq!(thread.live_pane_id.as_deref(), Some("%1"));
    }

    pub(crate) fn sync_sidebar_selection_recovers_collapsed_thread_to_folder_key() {
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

    pub(crate) fn sync_sidebar_selection_falls_back_to_first_visible_item_when_selected_key_is_filtered_out(
    ) {
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

    pub(crate) fn visible_sidebar_items_sequence_stays_stable_across_expand_and_search() {
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
}
