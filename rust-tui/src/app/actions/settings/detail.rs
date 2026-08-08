mod enter {
    use super::super::super::*;
    use crate::app::state::RelayView;

    impl App {
        pub fn enter_settings_detail(&mut self) {
            let Some(kind) = self.current_settings_detail_kind() else {
                self.return_to_settings_list();
                return;
            };
            let route_query = self.settings_search.clone();
            self.active_settings_detail = Some(kind);
            self.prepare_settings_detail(kind);
            super::search_route::apply_settings_search_route(self, kind, &route_query);
            self.settings_focus = SettingsFocus::Detail;
            self.settings_searching = false;
            self.dirty = true;
        }

        fn return_to_settings_list(&mut self) {
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.settings_searching = false;
            self.dirty = true;
        }

        fn prepare_settings_detail(&mut self, kind: SettingsDetailKind) {
            match kind {
                SettingsDetailKind::Theme => self.prepare_theme_detail(),
                SettingsDetailKind::Language => self.prepare_language_detail(),
                SettingsDetailKind::Relay => self.prepare_relay_detail(),
                SettingsDetailKind::Telegram => self.prepare_telegram_detail(),
                SettingsDetailKind::CodexSettings => self.reset_codex_settings_detail(),
                SettingsDetailKind::Sound => self.sound_settings_selected = 0,
                SettingsDetailKind::Trash => {}
                _ => {}
            }
        }

        fn prepare_theme_detail(&mut self) {
            self.preview.theme_before_preview = Some(self.config.theme.clone());
            self.theme_selected = Self::available_themes()
                .iter()
                .position(|(name, _)| *name == self.config.theme)
                .unwrap_or(0);
        }

        fn prepare_language_detail(&mut self) {
            self.locale = crate::i18n::Locale::from_str(&self.config.language);
            let locales = Self::available_locales();
            self.language_selected = locales
                .iter()
                .position(|l| l.as_str() == self.config.language)
                .unwrap_or(0);
        }

        fn prepare_relay_detail(&mut self) {
            self.relay_view = RelayView::AgentList;
            self.relay_selected_agent = self
                .relay_selected_agent
                .min(self.config.agents.len().saturating_sub(1));
            self.relay_selected_provider = 0;
            self.relay_edit_field = 0;
            self.relay_editing = false;
            self.relay_edit_buffer.clear();
            self.clear_relay_popup_state();
        }

        fn prepare_telegram_detail(&mut self) {
            self.telegram_selected_field = 0;
            self.telegram_editing = false;
            self.telegram_edit_buffer.clear();
        }
    }
}
mod kind {
    use super::super::super::*;

    impl App {
        pub fn current_settings_item_id(&self) -> Option<&'static str> {
            self.filtered_settings_items()
                .get(self.settings_selected)
                .map(|(id, _, _, _, _)| *id)
        }

        pub fn current_settings_detail_kind(&self) -> Option<SettingsDetailKind> {
            if self.settings_focus == SettingsFocus::Detail {
                return self.active_settings_detail;
            }
            self.settings_detail_kind_from_item_id(self.current_settings_item_id()?)
        }

        fn settings_detail_kind_from_item_id(
            &self,
            item_id: &'static str,
        ) -> Option<SettingsDetailKind> {
            Some(match item_id {
                "theme" => SettingsDetailKind::Theme,
                "auto_refresh" => SettingsDetailKind::AutoRefresh,
                "codex_settings" => SettingsDetailKind::CodexSettings,
                "claude_full_access" => SettingsDetailKind::ClaudeFullAccess,
                "sound" => SettingsDetailKind::Sound,
                "relay" => SettingsDetailKind::Relay,
                "telegram" => SettingsDetailKind::Telegram,
                "preview_mode" => SettingsDetailKind::PreviewMode,
                "display_mode" => SettingsDetailKind::DisplayMode,
                "trash" => SettingsDetailKind::Trash,
                "language" => SettingsDetailKind::Language,
                "version" => SettingsDetailKind::Version,
                _ => return None,
            })
        }
    }
}
mod open {
    use super::super::super::*;

    impl App {
        pub fn toggle_settings(&mut self) {
            self.settings_open = !self.settings_open;
            if self.settings_open {
                self.mode = Mode::Settings;
                self.reset_settings_list_state(false);
            } else {
                self.close_settings();
                return;
            }
            self.dirty = true;
        }

        pub fn open_settings_search(&mut self) {
            self.settings_open = true;
            self.mode = Mode::Settings;
            self.reset_settings_list_state(true);
            self.dirty = true;
        }

        pub fn close_settings(&mut self) {
            self.restore_settings_detail_preview_state();
            self.settings_open = false;
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.settings_searching = false;
            self.settings_search.clear();
            self.mode = Mode::Normal;
            self.dirty = true;
        }

        fn reset_settings_list_state(&mut self, searching: bool) {
            self.settings_selected = 0;
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.settings_searching = searching;
            self.settings_search.clear();
        }
    }
}
mod restore {
    use super::super::super::*;

    impl App {
        pub fn leave_settings_detail(&mut self) {
            self.restore_settings_detail_preview_state();
            if self.active_settings_detail == Some(SettingsDetailKind::CodexSettings) {
                self.reset_codex_settings_detail();
            }
            self.settings_focus = SettingsFocus::List;
            self.active_settings_detail = None;
            self.relay_editing = false;
            self.relay_edit_buffer.clear();
            self.clear_relay_popup_state();
            self.telegram_editing = false;
            self.telegram_edit_buffer.clear();
            self.dirty = true;
        }

        pub fn restore_settings_detail_preview_state(&mut self) {
            match self.current_settings_detail_kind() {
                Some(SettingsDetailKind::Theme) => {
                    if let Some(prev) = self.preview.theme_before_preview.take() {
                        self.theme = crate::theme::Theme::by_name(&prev);
                        self.clear_preview_render_caches();
                    }
                }
                Some(SettingsDetailKind::Language) => {
                    self.locale = crate::i18n::Locale::from_str(&self.config.language);
                }
                _ => {}
            }
        }

        pub(in crate::app::actions::settings::detail) fn reset_codex_settings_detail(&mut self) {
            self.codex_settings_view = crate::app::state::CodexSettingsView::Categories;
            self.codex_settings_category_selected = 0;
            self.codex_settings_selected = 0;
        }
    }
}
mod search_route {
    use super::super::super::*;
    use crate::app::state::{CodexSettingsView, RelayView, SettingsDetailKind};

    pub(in crate::app::actions::settings::detail) fn apply_settings_search_route(
        app: &mut App,
        kind: SettingsDetailKind,
        query: &str,
    ) {
        let query = query.trim();
        if query.is_empty() {
            return;
        }

        match kind {
            SettingsDetailKind::Relay => apply_relay_route(app, query),
            SettingsDetailKind::CodexSettings => apply_codex_settings_route(app, query),
            _ => {}
        }
    }

    fn apply_relay_route(app: &mut App, query: &str) {
        let Some(agent_idx) = matching_agent_index(app, query) else {
            return;
        };

        app.relay_selected_agent = agent_idx;
        app.relay_selected_provider = app
            .config
            .agents
            .get(agent_idx)
            .and_then(|agent| agent.active_provider)
            .unwrap_or(0);
        app.relay_view = RelayView::ProviderList;

        if let Some(provider_idx) = matching_provider_index(app, agent_idx, query) {
            app.relay_selected_provider = provider_idx;
            app.relay_view = RelayView::DetailPane;
        }
    }

    fn matching_agent_index(app: &App, query: &str) -> Option<usize> {
        app.config.agents.iter().position(|agent| {
            query
                .split_whitespace()
                .any(|token| token.eq_ignore_ascii_case(agent.name.as_str()))
        })
    }

    fn matching_provider_index(app: &App, agent_idx: usize, query: &str) -> Option<usize> {
        let agent = app.config.agents.get(agent_idx)?;
        let agent_name = agent.name.as_str();
        let extra_tokens = query
            .split_whitespace()
            .filter(|token| !is_relay_route_token(token, agent_name))
            .collect::<Vec<_>>();
        if extra_tokens.is_empty() {
            return None;
        }

        agent.providers.iter().position(|provider| {
            let blob = format!(
                "{} {} {}",
                provider.label, provider.base_url, provider.provider_key
            );
            extra_tokens
                .iter()
                .all(|token| crate::text_match::contains_ignore_case(&blob, token))
        })
    }

    fn is_relay_route_token(token: &str, agent_name: &str) -> bool {
        token.eq_ignore_ascii_case("relay")
            || token.eq_ignore_ascii_case("provider")
            || token.eq_ignore_ascii_case("providers")
            || token.eq_ignore_ascii_case("settings")
            || token.eq_ignore_ascii_case(agent_name)
    }

    fn apply_codex_settings_route(app: &mut App, query: &str) {
        let Some(view) = codex_view_from_query(query) else {
            return;
        };
        app.codex_settings_view = view;
        app.codex_settings_category_selected = view.category_index();
        app.codex_settings_selected = 0;
    }

    fn codex_view_from_query(query: &str) -> Option<CodexSettingsView> {
        let q = query.to_ascii_lowercase();
        if q.contains("status") || q.contains("statusline") {
            Some(CodexSettingsView::StatusLine)
        } else if q.contains("prompt") {
            Some(CodexSettingsView::Prompts)
        } else if q.contains("preview") || q.contains("summary") {
            Some(CodexSettingsView::Preview)
        } else if q.contains("cli") || q.contains("version") || q.contains("update") {
            Some(CodexSettingsView::Cli)
        } else if q.contains("runtime")
            || q.contains("permission")
            || q.contains("yolo")
            || q.contains("fast")
            || q.contains("goal")
            || q.contains("web")
            || q.contains("search")
            || q.contains("multi")
        {
            Some(CodexSettingsView::Runtime)
        } else {
            None
        }
    }
}
