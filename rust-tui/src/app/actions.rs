use super::App;
use super::state::Mode;
use crate::tree;
use std::path::PathBuf;

impl App {
    pub fn toggle_tree(&mut self) {
        self.show_tree = !self.show_tree;
        if self.show_tree {
            if let Some(panel) = self.selected_panel() {
                let path = PathBuf::from(&panel.working_dir);
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
            self.file_tree = Some(tree::FileTree::new(home));
            self.mode = Mode::Tree;
            self.update_file_preview();
            self.dirty = true;
        }
    }

    pub fn close_tree(&mut self) {
        self.show_tree = false;
        self.file_tree = None;
        self.agent_launcher = None;
        self.mode = Mode::Normal;
        self.dirty = true;
    }

    pub fn open_agent_launcher(&mut self, target_dir: PathBuf) {
        self.agent_launcher =
            Some(tree::AgentLauncher::with_agents(target_dir, self.config.agents.clone()));
        self.mode = Mode::AgentLauncher;
        self.dirty = true;
    }

    pub fn close_agent_launcher(&mut self) {
        self.agent_launcher = None;
        self.mode = Mode::Tree;
        self.dirty = true;
    }

    pub fn update_file_preview(&mut self) {
        if let Some(ref tree) = self.file_tree {
            if let Some(entry) = tree.selected() {
                let path = &entry.path;
                let preview_type = tree::PreviewType::from_path(path);

                if preview_type.is_text() {
                    self.file_preview_path = Some(path.clone());
                    self.file_preview_content = Self::load_text_file(path, 100);
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

    pub fn settings_items(&self) -> Vec<(&str, String, &str, bool)> {
        vec![
            ("Theme", self.config.theme.clone(), "Color scheme", true),
            (
                "Auto Refresh",
                if self.config.auto_refresh {
                    "On".to_string()
                } else {
                    "Off".to_string()
                },
                "Auto-refresh panel list",
                true,
            ),
            (
                "Refresh Interval",
                format!("{}s", self.config.refresh_interval),
                "Seconds between refreshes",
                false,
            ),
            (
                "Version",
                "0.5.0".to_string(),
                "pad - Agent Panel Manager",
                false,
            ),
        ]
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
            ("rose-pine", "Rose Pine"),
        ]
    }
}
