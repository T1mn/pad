use super::*;
use crate::sidebar::{SidebarFolder, SidebarItem, SidebarThread};
use std::sync::Arc;

pub(crate) fn shimmer_preserves_text_content() {
    let text = "rust-tui";
    let rendered: String = animation::shimmer_spans(
        text,
        ratatui::style::Color::White,
        ratatui::style::Color::Cyan,
        ratatui::style::Color::Black,
    )
    .into_iter()
    .map(|span| span.content.to_string())
    .collect();
    assert_eq!(rendered, text);
}

pub(crate) fn waiting_threads_do_not_breathe() {
    assert!(animation::thread_badge_breathes(
        &crate::model::AgentState::Busy
    ));
    assert!(!animation::thread_badge_breathes(
        &crate::model::AgentState::Waiting
    ));
    assert!(!animation::thread_badge_breathes(
        &crate::model::AgentState::Idle
    ));
}

pub(crate) fn visible_thread_count_ignores_folder_rows() {
    let folder = SidebarFolder {
        key: "folder:/tmp".into(),
        path: "/tmp".into(),
        label: "tmp".into(),
        updated_at: 0,
        threads: Vec::new(),
    };
    let thread = SidebarThread {
        key: "thread:1".into(),
        folder_key: folder.key.clone(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: crate::model::AgentType::Codex,
        session_id: Some("session-1".into()),
        transcript_path: None,
        session_provider_name: None,
        title: "Test".into(),
        upstream_title: None,
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 0,
        sort_updated_at: 0,
        live_pane_id: None,
        live_location: None,
        state: crate::model::AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };
    let items = vec![
        SidebarItem::folder(folder.summary()),
        SidebarItem::Thread(Arc::new(thread)),
    ];

    assert_eq!(
        crate::app::state::VisibleSidebarStats::from_items(&items).thread_count,
        1
    );
}

pub(crate) fn visible_thread_jump_badges_ignore_folders_and_cap_at_nine() {
    let folder = SidebarFolder {
        key: "folder:/tmp".into(),
        path: "/tmp".into(),
        label: "tmp".into(),
        updated_at: 0,
        threads: Vec::new(),
    };
    let thread = |index: usize| SidebarThread {
        key: format!("thread:{index}"),
        folder_key: folder.key.clone(),
        working_dir: "/tmp".into(),
        folder_label: "tmp".into(),
        agent_type: crate::model::AgentType::Codex,
        session_id: Some(format!("session-{index}")),
        transcript_path: None,
        session_provider_name: None,
        title: format!("Test {index}"),
        upstream_title: None,
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 0,
        sort_updated_at: 0,
        live_pane_id: None,
        live_location: None,
        state: crate::model::AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    };
    let mut items = vec![SidebarItem::folder(folder.summary())];
    for index in 1..=10 {
        items.push(SidebarItem::Thread(Arc::new(thread(index))));
    }

    let badges = viewport::visible_thread_jump_badges(&items);
    assert_eq!(badges[0], None);
    assert_eq!(badges[1], Some(1));
    assert_eq!(badges[9], Some(9));
    assert_eq!(badges[10], None);
}

pub(crate) mod folder_row {
    use super::super::folder_row::{count_style, folder_label_style};
    use crate::theme::Theme;
    use ratatui::style::Modifier;

    pub(crate) fn folder_label_uses_readable_text_without_dim() {
        let theme = Theme::default();
        let style = folder_label_style(false, false, &theme, theme.bg);

        assert_eq!(style.fg, Some(theme.fg));
        assert!(!style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    pub(crate) fn folder_count_uses_accent_without_dim() {
        let theme = Theme::default();
        let style = count_style(false, false, &theme, theme.bg);

        assert_eq!(style.fg, Some(theme.accent));
        assert!(!style.add_modifier.contains(Modifier::DIM));
    }
}

pub(crate) mod thread_row {
    use super::super::thread_row::format_jump_badge;

    pub(crate) fn jump_badge_is_fixed_width_and_limited_to_nine() {
        assert_eq!(format_jump_badge(Some(1), 4), "#1  ");
        assert_eq!(format_jump_badge(Some(9), 4), "#9  ");
        assert_eq!(format_jump_badge(Some(10), 4), "    ");
        assert_eq!(format_jump_badge(None, 4), "    ");
    }
}

pub(crate) mod viewport_tests {
    use super::super::viewport::render_window;

    pub(crate) fn keeps_selected_near_middle_when_possible() {
        let range = render_window(20, Some(10), 5, |_| 1);

        assert_eq!(range, 8..13);
    }

    pub(crate) fn fills_from_top_when_selection_is_near_start() {
        let range = render_window(20, Some(1), 5, |_| 1);

        assert_eq!(range, 0..5);
    }

    pub(crate) fn respects_tall_thread_rows() {
        let range = render_window(20, Some(5), 6, |idx| if idx % 2 == 0 { 1 } else { 2 });

        assert!(range.contains(&5));
        let total_height: usize = range
            .clone()
            .map(|idx| if idx % 2 == 0 { 1 } else { 2 })
            .sum();
        assert!(total_height <= 6);
    }
}

pub(crate) mod width {
    use super::super::width::preferred_panel_width;
    use crate::app::state::PreferredPanelWidthCache;
    use crate::app::App;
    use crate::model::{AgentPanel, AgentState, AgentType};

    pub(crate) fn preferred_panel_width_keeps_short_name_visible() {
        let mut app = App::new();
        app.panels.push(AgentPanel {
            session: "0".into(),
            window: "kanban".into(),
            window_index: "1".into(),
            pane: "1".into(),
            pane_id: "%1".into(),
            agent_type: AgentType::Codex,
            working_dir: "/tmp/rust-tui".into(),
            is_active: true,
            state: AgentState::Busy,
            transcript_path: None,
            cached_preview_turns: Default::default(),
            session_cache_state: None,
            agent_session_id: None,
            last_user_prompt: None,
            last_assistant_message: None,
            has_unread_stop: false,
        });

        assert!(preferred_panel_width(&mut app) >= 13);
    }

    pub(crate) fn preferred_panel_width_cache_clears_on_sidebar_invalidation() {
        let mut app = App::new();
        app.sidebar.visible_sidebar_items_dirty = false;
        app.sidebar.preferred_panel_width_cache = Some(PreferredPanelWidthCache {
            width: 33,
            locale: app.locale,
            thread_list_view: app.thread_list_view(),
            live_only: app.showing_live_sessions(),
            manual_width: app.config.display.agent_panel_width,
        });

        assert_eq!(preferred_panel_width(&mut app), 33);

        app.invalidate_sidebar_visible_cache();

        assert!(app.sidebar.preferred_panel_width_cache.is_none());
    }

    pub(crate) fn thread_width_grows_with_long_titles() {
        let short = super::super::width::thread_item_width("短标题");
        let long = super::super::width::thread_item_width(
            "这是一个比较长的会话标题，用来确认左侧 pane 会根据标题长度自动变宽",
        );

        assert!(long > short);
        assert!(long > 46);
    }

    pub(crate) fn manual_width_is_used_as_minimum() {
        let mut app = App::new();
        app.config.display.agent_panel_width = Some(70);

        assert!(preferred_panel_width(&mut app) >= 70);
    }
}
