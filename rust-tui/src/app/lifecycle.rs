use super::{
    App, CodexSettingsView, Mode, PreviewState, RelayPopupMode, RelayView, SettingsFocus,
    SidebarState,
};
use crate::theme::{Config, Theme};
use ratatui::widgets::TableState;
use std::collections::HashSet;
use std::time::{Duration, Instant};

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let config = Config::load();
        let display_session_scope = config.display.session_scope.clone();
        let locale = crate::i18n::Locale::from_str(&config.language);
        let theme = Theme::by_name(&config.theme);

        Self {
            panels: Vec::new(),
            table_state,
            mode: Mode::Normal,
            last_refresh: Instant::now(),
            search_query: String::new(),
            is_searching: false,
            sidebar: SidebarState::new(display_session_scope),
            preview: PreviewState::new(),
            settings_open: false,
            config,
            locale,
            theme,
            settings_selected: 0,
            settings_focus: SettingsFocus::List,
            active_settings_detail: None,
            theme_selected: 0,
            language_selected: 0,
            scan_in_progress: false,
            scan_rx: None,
            hook_rx: None,
            refresh_after_attach: false,
            should_quit: false,
            dirty: true,
            same_session_attached: false,
            same_session_trace_id: None,
            saved_tmux_bindings: Vec::new(),
            saved_tmux_status: None,
            saved_tmux_status_target: None,
            fuzzy_picker: None,
            fuzzy_from_normal: false,
            relay_selected_agent: 0,
            relay_selected_provider: 0,
            relay_editing: false,
            relay_edit_field: 0,
            relay_edit_buffer: String::new(),
            relay_view: RelayView::AgentList,
            relay_popup_mode: RelayPopupMode::None,
            relay_popup_selected: 0,
            relay_popup_field: 0,
            relay_popup_editing: false,
            relay_popup_buffer: String::new(),
            settings_search: String::new(),
            settings_searching: false,
            delayed_scan_at: None,
            needs_clear: false,
            provider_test_in_progress: false,
            provider_test_pending_count: 0,
            provider_test_sort_agent_on_complete: None,
            provider_test_rx: None,
            codex_cli_check_in_progress: false,
            codex_cli_check_rx: None,
            codex_cli_update_in_progress: false,
            codex_cli_update_rx: None,
            codex_cli_version_info: None,
            title_summary_tx: None,
            title_summary_rx: None,
            title_summary_in_flight: HashSet::new(),
            agent_style_selected: 0,
            codex_settings_view: CodexSettingsView::Categories,
            codex_settings_category_selected: 0,
            codex_settings_selected: 0,
            sound_settings_selected: 0,
            telegram_selected_field: 0,
            telegram_editing: false,
            telegram_edit_buffer: String::new(),
            busy_animation_frame: 0,
            last_busy_animation_tick: Instant::now(),
            last_draw_elapsed: Duration::default(),
            frame_budget_exceeded: false,
            deferred_hook_events: Vec::new(),
            deferred_scan_result: None,
            notification_inbox: crate::notification_inbox::load(),
            notification_inbox_selected: 0,
            relay_config_last_poll_at: Instant::now(),
            relay_config_source_path: None,
            relay_config_source_modified_ms: None,
            relay_config_source_len: None,
            pending_external_relay_reload: false,
        }
    }
}
