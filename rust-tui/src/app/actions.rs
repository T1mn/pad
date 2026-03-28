use super::state::Mode;
use super::{App, PendingThreadAction, ThreadActionKind, ThreadMetaEditKind};
use crate::fuzzy::{scan_directories, FuzzyPicker};
use crate::i18n::Locale;
use crate::log_debug;
use crate::model::AgentType;
use crate::sidebar::{SidebarItem, SidebarThread};
use crate::tree;
use std::path::PathBuf;

impl App {
    pub fn toggle_tree(&mut self) {
        self.show_tree = !self.show_tree;
        self.focus_panel();
        if self.show_tree {
            if let Some(thread) = self.selected_preview_thread() {
                let path = PathBuf::from(&thread.working_dir);
                if path.exists() {
                    self.file_tree = Some(tree::FileTree::new(path));
                    self.mode = Mode::Tree;
                    self.update_file_preview();
                }
            }
        } else {
            self.file_tree = None;
            self.file_preview_path = None;
            self.file_preview_content.clear();
            self.mode = Mode::Normal;
        }
        self.dirty = true;
    }

    pub fn open_tree_in_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.show_tree = true;
            self.focus_panel();
            self.file_tree = Some(tree::FileTree::new(home));
            self.mode = Mode::Tree;
            self.update_file_preview();
            self.dirty = true;
        }
    }

    pub fn close_tree(&mut self) {
        self.show_tree = false;
        self.focus_panel();
        self.file_tree = None;
        self.agent_launcher = None;
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn open_agent_launcher(&mut self, target_dir: PathBuf) {
        let agent_tuples: Vec<(String, String)> = self
            .config
            .agents
            .iter()
            .map(|a| (a.name.clone(), a.cmd.clone()))
            .collect();
        self.agent_launcher = Some(tree::AgentLauncher::with_agents(target_dir, agent_tuples));
        self.mode = Mode::AgentLauncher;
        self.dirty = true;
    }

    pub fn close_agent_launcher(&mut self) {
        let was_fuzzy = self.fuzzy_from_normal;
        self.agent_launcher = None;
        self.fuzzy_from_normal = false;
        if was_fuzzy || !self.show_tree {
            self.mode = Mode::Normal;
        } else {
            self.mode = Mode::Tree;
        }
        self.dirty = true;
    }

    pub fn open_fuzzy_picker(&mut self) {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        let items = scan_directories(&home, 3);
        self.fuzzy_picker = Some(FuzzyPicker::new(items));
        self.fuzzy_from_normal = true;
        self.mode = Mode::FuzzyPicker;
        self.dirty = true;
    }

    pub fn close_fuzzy_picker(&mut self) {
        self.fuzzy_picker = None;
        self.fuzzy_from_normal = false;
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn update_file_preview(&mut self) {
        if let Some(ref tree) = self.file_tree {
            if let Some(entry) = tree.selected() {
                let path = &entry.path;
                let preview_type = tree::PreviewType::from_path(path);

                if preview_type.is_text() {
                    self.file_preview_path = Some(path.clone());
                    self.file_preview_content = Self::load_text_file(path, 500);
                    self.file_preview_scroll = 0;
                } else if preview_type.is_image() {
                    self.file_preview_path = Some(path.clone());
                    self.file_preview_content = format!(
                        "🖼️  Image file: {}\n\n(Use terminal image viewer like 'icat' to preview images)",
                        path.display()
                    );
                } else if preview_type == tree::PreviewType::Directory {
                    self.file_preview_path = Some(path.clone());
                    self.file_preview_content = Self::load_directory_info(path);
                } else {
                    self.file_preview_path = Some(path.clone());
                    self.file_preview_content = format!(
                        "📦 Binary file: {}\n\nSize: {}\nType: {:?}",
                        path.display(),
                        Self::format_file_size(path),
                        preview_type
                    );
                }
            } else {
                self.file_preview_path = None;
                self.file_preview_content = "No file selected".to_string();
            }
        }
        self.dirty = true;
    }

    pub fn load_text_file(path: &PathBuf, max_lines: usize) -> String {
        use std::io::BufRead;

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return format!("Error opening file: {}", e),
        };

        let reader = std::io::BufReader::new(file);
        let mut content = String::new();
        let mut line_count = 0;

        for line in reader.lines() {
            if line_count >= max_lines {
                content.push_str("\n... (truncated)");
                break;
            }
            match line {
                Ok(l) => {
                    content.push_str(&l);
                    content.push('\n');
                    line_count += 1;
                }
                Err(e) => {
                    content.push_str(&format!("\n[Error reading line: {}]", e));
                    break;
                }
            }
        }

        content
    }

    pub fn load_directory_info(path: &PathBuf) -> String {
        let mut content = format!("📁 Directory: {}\n\n", path.display());

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut count = 0;
            for entry in entries.flatten() {
                if count >= 50 {
                    content.push_str("\n... (more items)");
                    break;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata();
                let icon = if metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                    "📁"
                } else {
                    "📄"
                };
                content.push_str(&format!("{} {}\n", icon, name));
                count += 1;
            }
            if count == 0 {
                content.push_str("(empty directory)");
            }
        } else {
            content.push_str("(cannot read directory)");
        }

        content
    }

    pub fn format_file_size(path: &PathBuf) -> String {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let size = metadata.len();
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else if size < 1024 * 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }

    pub fn delete_panel(&mut self, panel: &crate::model::AgentPanel) {
        self.pending_sidebar_selection_index = self.table_state.selected();
        let _ = std::process::Command::new("tmux")
            .args(["kill-pane", "-t", &panel.pane_id])
            .output();
        self.refresh_panels();
    }

    pub fn refresh_panels(&mut self) {
        if !self.scan_in_progress {
            self.trigger_async_scan();
        }
    }

    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.mode = Mode::Settings;
            self.settings_selected = 0;
        } else {
            self.mode = Mode::Normal;
        }
        self.dirty = true;
    }

    pub fn open_theme_selector(&mut self) {
        self.theme_before_preview = Some(self.config.theme.clone());
        self.theme_selector_open = true;
        self.mode = Mode::ThemeSelector;
        self.theme_selected = 0;
        self.dirty = true;
    }

    pub fn close_theme_selector(&mut self) {
        // Restore theme to what it was before preview
        if let Some(ref prev) = self.theme_before_preview.take() {
            self.theme = crate::theme::Theme::by_name(prev);
        }
        self.theme_selector_open = false;
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    pub fn available_locales() -> Vec<crate::i18n::Locale> {
        use crate::i18n::Locale;
        vec![
            Locale::ZhCN,
            Locale::ZhTW,
            Locale::En,
            Locale::Ja,
            Locale::De,
            Locale::Fr,
        ]
    }

    pub fn toggle_archived_threads_view(&mut self) {
        self.archived_threads_view = !self.archived_threads_view;
        self.pending_thread_action = None;
        self.pending_sidebar_selection_index = None;
        self.mode = Mode::Normal;
        self.selected_sidebar_key = None;
        self.table_state.select(None);
        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.focus_panel();
        self.dirty = true;
    }

    pub fn request_archive_selected_thread(&mut self) -> bool {
        let Some(thread) = self.selected_thread_action_target(false) else {
            return false;
        };
        self.open_thread_action_confirm(thread, ThreadActionKind::Archive);
        true
    }

    pub fn request_unarchive_selected_thread(&mut self) -> bool {
        let Some(thread) = self.selected_thread_action_target(true) else {
            return false;
        };
        self.open_thread_action_confirm(thread, ThreadActionKind::Unarchive);
        true
    }

    pub fn open_thread_action_confirm(&mut self, thread: SidebarThread, kind: ThreadActionKind) {
        self.pending_thread_action = Some(PendingThreadAction { thread, kind });
        self.thread_meta_editing = false;
        self.thread_meta_target = None;
        self.thread_meta_buffer.clear();
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
    }

    pub fn close_thread_action_confirm(&mut self) {
        self.pending_thread_action = None;
        self.thread_meta_editing = false;
        self.thread_meta_target = None;
        self.thread_meta_buffer.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn confirm_thread_action(&mut self) -> bool {
        let Some(action) = self.pending_thread_action.clone() else {
            self.mode = Mode::Normal;
            self.dirty = true;
            return false;
        };
        self.pending_thread_action = None;
        self.mode = Mode::Normal;

        let Some(session_id) = action.thread.session_id.as_deref() else {
            self.dirty = true;
            return false;
        };

        let result = match action.kind {
            ThreadActionKind::Archive => match action.thread.agent_type {
                AgentType::Codex => crate::codex_state::archive_thread(session_id),
                AgentType::Claude => crate::claude_history::archive_thread(session_id),
                AgentType::Gemini => crate::gemini_history::archive_thread(session_id),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "archive is not supported for this agent type",
                )),
            },
            ThreadActionKind::Unarchive => match action.thread.agent_type {
                AgentType::Codex => crate::codex_state::unarchive_thread(session_id),
                AgentType::Claude => crate::claude_history::unarchive_thread(session_id),
                AgentType::Gemini => crate::gemini_history::unarchive_thread(session_id),
                _ => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "restore is not supported for this agent type",
                )),
            },
        };
        let ok = result.is_ok();

        match &result {
            Ok(()) => self.show_action_toast(
                success_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                &thread_action_subject(&action.thread),
            ),
            Err(err) => self.show_action_toast(
                failure_toast_title(self.locale, action.kind, action.thread.agent_type.clone()),
                &err.to_string(),
            ),
        }

        self.invalidate_sidebar_cache();
        self.sync_sidebar_selection();
        self.invalidate_preview();
        self.focus_panel();
        self.dirty = true;
        ok
    }

    pub fn open_thread_title_editor(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };

        self.pending_thread_action = None;
        self.thread_meta_editing = true;
        self.thread_meta_edit_kind = ThreadMetaEditKind::Title;
        self.thread_meta_buffer = thread.title.clone();
        self.thread_meta_target = Some(thread);
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
        true
    }

    pub fn open_thread_tags_editor(&mut self) -> bool {
        let Some(SidebarItem::Thread(thread)) = self.selected_sidebar_item() else {
            return false;
        };

        self.pending_thread_action = None;
        self.thread_meta_editing = true;
        self.thread_meta_edit_kind = ThreadMetaEditKind::Tags;
        self.thread_meta_buffer = thread.tags.join(", ");
        self.thread_meta_target = Some(thread);
        self.mode = Mode::ThreadActionConfirm;
        self.dirty = true;
        true
    }

    pub fn cancel_thread_meta_edit(&mut self) {
        self.thread_meta_editing = false;
        self.thread_meta_target = None;
        self.thread_meta_buffer.clear();
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn commit_thread_meta_edit(&mut self) -> bool {
        let Some(thread) = self.thread_meta_target.clone() else {
            self.cancel_thread_meta_edit();
            return false;
        };

        let input = self.thread_meta_buffer.trim().to_string();
        let result = match self.thread_meta_edit_kind {
            ThreadMetaEditKind::Title => self.persist_thread_title_override(&thread, &input),
            ThreadMetaEditKind::Tags => {
                let tags = parse_thread_tags(&input);
                self.persist_thread_tags(&thread, &tags)
            }
        };

        let ok = result.is_ok();
        if ok {
            let (title, content) =
                thread_meta_toast(self.locale, self.thread_meta_edit_kind, &input);
            self.show_action_toast(title, &content);
        } else if let Err(err) = result {
            self.show_action_toast(thread_meta_save_failed_title(self.locale), &err.to_string());
        }

        if ok {
            self.invalidate_sidebar_cache();
            self.sync_sidebar_selection();
            self.invalidate_preview();
        }
        self.cancel_thread_meta_edit();
        ok
    }

    fn persist_thread_title_override(
        &mut self,
        thread: &SidebarThread,
        title: &str,
    ) -> std::io::Result<()> {
        let session_id = thread.session_id.as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "selected thread is missing session id",
            )
        })?;
        log_debug!(
            "thread_meta: title override save requested thread={} title={}",
            thread.key,
            title
        );
        crate::thread_meta::upsert_thread_meta(
            &thread.agent_type.to_string(),
            session_id,
            Some(title),
            thread.note.as_deref(),
            thread.pinned,
        )
    }

    fn persist_thread_tags(
        &mut self,
        thread: &SidebarThread,
        tags: &[String],
    ) -> std::io::Result<()> {
        let session_id = thread.session_id.as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "selected thread is missing session id",
            )
        })?;
        log_debug!(
            "thread_meta: tags save requested thread={} tags={:?}",
            thread.key,
            tags
        );
        crate::thread_meta::replace_thread_tags(&thread.agent_type.to_string(), session_id, tags)
    }

    fn selected_thread_action_target(&mut self, archived: bool) -> Option<SidebarThread> {
        match self.selected_sidebar_item()? {
            SidebarItem::Thread(thread)
                if matches!(
                    thread.agent_type,
                    AgentType::Codex | AgentType::Claude | AgentType::Gemini
                ) && thread.archived == archived
                    && thread.session_id.is_some() =>
            {
                Some(thread)
            }
            _ => None,
        }
    }

    pub fn open_language_selector(&mut self) {
        let locales = Self::available_locales();
        self.language_selected = locales.iter().position(|l| *l == self.locale).unwrap_or(0);
        self.mode = Mode::LanguageSelector;
        self.dirty = true;
    }

    pub fn close_language_selector(&mut self) {
        // Revert to saved locale
        self.locale = crate::i18n::Locale::from_str(&self.config.language);
        self.mode = Mode::Settings;
        self.dirty = true;
    }

    /// Returns (id, value, name_i18n_key, desc_i18n_key, editable)
    pub fn settings_items(&self) -> Vec<(&'static str, String, &'static str, &'static str, bool)> {
        let l = self.locale;
        let preview_mode = match self.config.preview.mode.as_str() {
            "tmux" => crate::i18n::t(l, "settings.preview_mode_tmux"),
            "session" => crate::i18n::t(l, "settings.preview_mode_session"),
            _ => crate::i18n::t(l, "settings.preview_mode_auto"),
        };
        let display_mode = match self.config.display.session_scope.as_str() {
            "all" => crate::i18n::t(l, "settings.display_mode_all"),
            _ => crate::i18n::t(l, "settings.display_mode_live"),
        };
        vec![
            (
                "theme",
                self.config.theme.clone(),
                "settings.theme",
                "settings.theme",
                true,
            ),
            (
                "auto_refresh",
                if self.config.auto_refresh {
                    crate::i18n::t(l, "settings.on").to_string()
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
                "settings.auto_refresh",
                "settings.auto_refresh",
                true,
            ),
            (
                "relay",
                crate::i18n::t(l, "settings.configure").to_string(),
                "settings.relay",
                "settings.relay",
                true,
            ),
            (
                "telegram",
                if self.config.telegram.enabled {
                    crate::i18n::t(l, "settings.on").to_string()
                } else {
                    crate::i18n::t(l, "settings.off").to_string()
                },
                "settings.telegram",
                "settings.telegram",
                true,
            ),
            (
                "agent_style",
                crate::i18n::t(l, "settings.configure").to_string(),
                "settings.agent_style",
                "settings.agent_style",
                true,
            ),
            (
                "preview_mode",
                preview_mode.to_string(),
                "settings.preview_mode",
                "settings.preview_mode",
                true,
            ),
            (
                "display_mode",
                display_mode.to_string(),
                "settings.display_mode",
                "settings.display_mode",
                true,
            ),
            (
                "language",
                self.locale.display_name().to_string(),
                "settings.language",
                "settings.language",
                true,
            ),
            (
                "refresh_interval",
                format!("{}s", self.config.refresh_interval),
                "settings.refresh_interval",
                "settings.refresh_interval",
                false,
            ),
            (
                "version",
                "0.6.0".to_string(),
                "settings.version",
                "settings.version",
                false,
            ),
        ]
    }

    pub fn filtered_settings_items(
        &self,
    ) -> Vec<(&'static str, String, &'static str, &'static str, bool)> {
        let items = self.settings_items();
        if self.settings_search.is_empty() {
            return items;
        }
        let query = self.settings_search.to_lowercase();
        let l = self.locale;
        items
            .into_iter()
            .filter(|(id, value, name_key, _, _)| {
                let name = crate::i18n::t(l, name_key);
                id.to_lowercase().contains(&query)
                    || value.to_lowercase().contains(&query)
                    || name.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("default", "Default"),
            ("dark", "Dark"),
            ("dracula", "Dracula"),
            ("nord", "Nord"),
            ("gruvbox", "Gruvbox"),
            ("catppuccin", "Catppuccin"),
            ("tokyo-night", "Tokyo Night"),
            ("monokai", "Monokai"),
            ("solarized-dark", "Solarized Dark"),
            ("solarized-light", "Solarized Light"),
            ("rose-pine", "Rose Pine"),
            ("one-dark", "One Dark"),
            ("github-light", "GitHub Light"),
            ("github-dark", "GitHub Dark"),
            ("everforest", "Everforest"),
        ]
    }
}

fn thread_action_subject(thread: &SidebarThread) -> String {
    if !thread.title.trim().is_empty() && thread.title != "untitled" {
        thread.title.clone()
    } else {
        thread
            .session_id
            .clone()
            .unwrap_or_else(|| thread.key.clone())
    }
}

fn success_toast_title(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: AgentType,
) -> &'static str {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, AgentType::Gemini) => "已在 pad 侧归档",
        (true, ThreadActionKind::Unarchive, AgentType::Gemini) => "已从 pad 侧恢复",
        (false, ThreadActionKind::Archive, AgentType::Gemini) => "Pad archived",
        (false, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad restored",
        (true, ThreadActionKind::Archive, _) => "已归档",
        (true, ThreadActionKind::Unarchive, _) => "已恢复",
        (false, ThreadActionKind::Archive, _) => "Archived",
        (false, ThreadActionKind::Unarchive, _) => "Restored",
    }
}

fn parse_thread_tags(input: &str) -> Vec<String> {
    input
        .split(|c: char| c == ',' || c == '\n' || c == ';')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_string())
        .collect()
}

fn thread_meta_save_failed_title(locale: Locale) -> &'static str {
    if matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja) {
        "保存失败"
    } else {
        "Save failed"
    }
}

fn thread_meta_toast(
    locale: Locale,
    kind: ThreadMetaEditKind,
    input: &str,
) -> (&'static str, String) {
    let empty_title = matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja);
    match kind {
        ThreadMetaEditKind::Title => {
            if input.is_empty() {
                if empty_title {
                    ("标题已清空", String::from("将回退到上游标题"))
                } else {
                    (
                        "Title cleared",
                        String::from("Will fall back to upstream title"),
                    )
                }
            } else if empty_title {
                ("标题已保存", input.to_string())
            } else {
                ("Title saved", input.to_string())
            }
        }
        ThreadMetaEditKind::Tags => {
            if input.is_empty() {
                if empty_title {
                    ("标签已清空", String::from("无标签"))
                } else {
                    ("Tags cleared", String::from("No tags"))
                }
            } else if empty_title {
                ("标签已保存", input.to_string())
            } else {
                ("Tags saved", input.to_string())
            }
        }
    }
}

fn failure_toast_title(
    locale: Locale,
    kind: ThreadActionKind,
    agent_type: AgentType,
) -> &'static str {
    match (is_cjk_locale(locale), kind, agent_type) {
        (true, ThreadActionKind::Archive, AgentType::Gemini) => "Pad 归档失败",
        (true, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad 恢复失败",
        (false, ThreadActionKind::Archive, AgentType::Gemini) => "Pad archive failed",
        (false, ThreadActionKind::Unarchive, AgentType::Gemini) => "Pad restore failed",
        (true, ThreadActionKind::Archive, _) => "归档失败",
        (true, ThreadActionKind::Unarchive, _) => "恢复失败",
        (false, ThreadActionKind::Archive, _) => "Archive Failed",
        (false, ThreadActionKind::Unarchive, _) => "Restore Failed",
    }
}

fn is_cjk_locale(locale: Locale) -> bool {
    matches!(locale, Locale::ZhCN | Locale::ZhTW | Locale::Ja)
}
