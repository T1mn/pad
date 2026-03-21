use ratatui::style::Color;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub status_fg: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub comment: Color,
    pub keyword: Color,
    pub string_color: Color,
    pub number: Color,
    pub mode_normal_bg: Color,
    pub mode_search_bg: Color,
    pub mode_tree_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::by_name("default")
    }
}

impl Theme {
    pub fn by_name(name: &str) -> Self {
        match name {
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "catppuccin" => Self::catppuccin(),
            "gruvbox" => Self::gruvbox(),
            "tokyo-night" => Self::tokyo_night(),
            "monokai" => Self::monokai(),
            "solarized-dark" => Self::solarized_dark(),
            "rose-pine" => Self::rose_pine(),
            "dark" => Self::dark(),
            _ => Self::default_theme(),
        }
    }

    fn default_theme() -> Self {
        Self {
            name: "default",
            bg: Color::Reset,
            fg: Color::Reset,
            accent: Color::Cyan,
            highlight_bg: Color::DarkGray,
            highlight_fg: Color::White,
            border: Color::DarkGray,
            border_focused: Color::Cyan,
            status_fg: Color::White,
            error: Color::Red,
            success: Color::Green,
            warning: Color::Yellow,
            comment: Color::DarkGray,
            keyword: Color::Magenta,
            string_color: Color::Green,
            number: Color::Cyan,
            mode_normal_bg: Color::Blue,
            mode_search_bg: Color::Yellow,
            mode_tree_bg: Color::Green,
        }
    }

    fn dark() -> Self {
        Self {
            name: "dark",
            bg: Color::Rgb(30, 30, 30),
            fg: Color::Rgb(204, 204, 204),
            accent: Color::Rgb(86, 182, 194),
            highlight_bg: Color::Rgb(60, 60, 60),
            highlight_fg: Color::White,
            border: Color::Rgb(68, 68, 68),
            border_focused: Color::Rgb(86, 182, 194),
            status_fg: Color::Rgb(204, 204, 204),
            error: Color::Rgb(244, 71, 71),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            comment: Color::Rgb(92, 99, 112),
            keyword: Color::Rgb(198, 120, 221),
            string_color: Color::Rgb(152, 195, 121),
            number: Color::Rgb(209, 154, 102),
            mode_normal_bg: Color::Rgb(86, 182, 194),
            mode_search_bg: Color::Rgb(229, 192, 123),
            mode_tree_bg: Color::Rgb(152, 195, 121),
        }
    }

    fn dracula() -> Self {
        Self {
            name: "dracula",
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            accent: Color::Rgb(189, 147, 249),
            highlight_bg: Color::Rgb(68, 71, 90),
            highlight_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(68, 71, 90),
            border_focused: Color::Rgb(189, 147, 249),
            status_fg: Color::Rgb(248, 248, 242),
            error: Color::Rgb(255, 85, 85),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            comment: Color::Rgb(98, 114, 164),
            keyword: Color::Rgb(255, 121, 198),
            string_color: Color::Rgb(241, 250, 140),
            number: Color::Rgb(189, 147, 249),
            mode_normal_bg: Color::Rgb(189, 147, 249),
            mode_search_bg: Color::Rgb(241, 250, 140),
            mode_tree_bg: Color::Rgb(80, 250, 123),
        }
    }

    fn nord() -> Self {
        Self {
            name: "nord",
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            accent: Color::Rgb(136, 192, 208),
            highlight_bg: Color::Rgb(67, 76, 94),
            highlight_fg: Color::Rgb(236, 239, 244),
            border: Color::Rgb(59, 66, 82),
            border_focused: Color::Rgb(136, 192, 208),
            status_fg: Color::Rgb(216, 222, 233),
            error: Color::Rgb(191, 97, 106),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            comment: Color::Rgb(76, 86, 106),
            keyword: Color::Rgb(180, 142, 173),
            string_color: Color::Rgb(163, 190, 140),
            number: Color::Rgb(180, 142, 173),
            mode_normal_bg: Color::Rgb(136, 192, 208),
            mode_search_bg: Color::Rgb(235, 203, 139),
            mode_tree_bg: Color::Rgb(163, 190, 140),
        }
    }

    fn catppuccin() -> Self {
        Self {
            name: "catppuccin",
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            accent: Color::Rgb(137, 180, 250),
            highlight_bg: Color::Rgb(49, 50, 68),
            highlight_fg: Color::Rgb(205, 214, 244),
            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(137, 180, 250),
            status_fg: Color::Rgb(205, 214, 244),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            comment: Color::Rgb(108, 112, 134),
            keyword: Color::Rgb(203, 166, 247),
            string_color: Color::Rgb(166, 227, 161),
            number: Color::Rgb(250, 179, 135),
            mode_normal_bg: Color::Rgb(137, 180, 250),
            mode_search_bg: Color::Rgb(249, 226, 175),
            mode_tree_bg: Color::Rgb(166, 227, 161),
        }
    }

    fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            bg: Color::Rgb(40, 40, 40),
            fg: Color::Rgb(235, 219, 178),
            accent: Color::Rgb(131, 165, 152),
            highlight_bg: Color::Rgb(80, 73, 69),
            highlight_fg: Color::Rgb(251, 241, 199),
            border: Color::Rgb(60, 56, 54),
            border_focused: Color::Rgb(131, 165, 152),
            status_fg: Color::Rgb(235, 219, 178),
            error: Color::Rgb(251, 73, 52),
            success: Color::Rgb(184, 187, 38),
            warning: Color::Rgb(250, 189, 47),
            comment: Color::Rgb(146, 131, 116),
            keyword: Color::Rgb(211, 134, 155),
            string_color: Color::Rgb(184, 187, 38),
            number: Color::Rgb(211, 134, 155),
            mode_normal_bg: Color::Rgb(131, 165, 152),
            mode_search_bg: Color::Rgb(250, 189, 47),
            mode_tree_bg: Color::Rgb(184, 187, 38),
        }
    }

    fn tokyo_night() -> Self {
        Self {
            name: "tokyo-night",
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(169, 177, 214),
            accent: Color::Rgb(122, 162, 247),
            highlight_bg: Color::Rgb(41, 46, 66),
            highlight_fg: Color::Rgb(192, 202, 245),
            border: Color::Rgb(41, 46, 66),
            border_focused: Color::Rgb(122, 162, 247),
            status_fg: Color::Rgb(169, 177, 214),
            error: Color::Rgb(247, 118, 142),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            comment: Color::Rgb(86, 95, 137),
            keyword: Color::Rgb(187, 154, 247),
            string_color: Color::Rgb(158, 206, 106),
            number: Color::Rgb(255, 158, 100),
            mode_normal_bg: Color::Rgb(122, 162, 247),
            mode_search_bg: Color::Rgb(224, 175, 104),
            mode_tree_bg: Color::Rgb(158, 206, 106),
        }
    }

    fn monokai() -> Self {
        Self {
            name: "monokai",
            bg: Color::Rgb(39, 40, 34),
            fg: Color::Rgb(248, 248, 242),
            accent: Color::Rgb(102, 217, 239),
            highlight_bg: Color::Rgb(73, 72, 62),
            highlight_fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(73, 72, 62),
            border_focused: Color::Rgb(102, 217, 239),
            status_fg: Color::Rgb(248, 248, 242),
            error: Color::Rgb(249, 38, 114),
            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(253, 151, 31),
            comment: Color::Rgb(117, 113, 94),
            keyword: Color::Rgb(249, 38, 114),
            string_color: Color::Rgb(230, 219, 116),
            number: Color::Rgb(174, 129, 255),
            mode_normal_bg: Color::Rgb(102, 217, 239),
            mode_search_bg: Color::Rgb(253, 151, 31),
            mode_tree_bg: Color::Rgb(166, 226, 46),
        }
    }

    fn solarized_dark() -> Self {
        Self {
            name: "solarized-dark",
            bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            accent: Color::Rgb(38, 139, 210),
            highlight_bg: Color::Rgb(7, 54, 66),
            highlight_fg: Color::Rgb(147, 161, 161),
            border: Color::Rgb(7, 54, 66),
            border_focused: Color::Rgb(38, 139, 210),
            status_fg: Color::Rgb(131, 148, 150),
            error: Color::Rgb(220, 50, 47),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            comment: Color::Rgb(88, 110, 117),
            keyword: Color::Rgb(108, 113, 196),
            string_color: Color::Rgb(42, 161, 152),
            number: Color::Rgb(203, 75, 22),
            mode_normal_bg: Color::Rgb(38, 139, 210),
            mode_search_bg: Color::Rgb(181, 137, 0),
            mode_tree_bg: Color::Rgb(133, 153, 0),
        }
    }

    fn rose_pine() -> Self {
        Self {
            name: "rose-pine",
            bg: Color::Rgb(25, 23, 36),
            fg: Color::Rgb(224, 222, 244),
            accent: Color::Rgb(196, 167, 231),
            highlight_bg: Color::Rgb(38, 35, 53),
            highlight_fg: Color::Rgb(224, 222, 244),
            border: Color::Rgb(38, 35, 53),
            border_focused: Color::Rgb(196, 167, 231),
            status_fg: Color::Rgb(224, 222, 244),
            error: Color::Rgb(235, 111, 146),
            success: Color::Rgb(156, 207, 216),
            warning: Color::Rgb(246, 193, 119),
            comment: Color::Rgb(110, 106, 134),
            keyword: Color::Rgb(196, 167, 231),
            string_color: Color::Rgb(246, 193, 119),
            number: Color::Rgb(235, 188, 186),
            mode_normal_bg: Color::Rgb(196, 167, 231),
            mode_search_bg: Color::Rgb(246, 193, 119),
            mode_tree_bg: Color::Rgb(156, 207, 216),
        }
    }
}

/// Config file management
#[derive(Clone, Debug)]
pub struct Config {
    pub theme: String,
    pub auto_refresh: bool,
    pub refresh_interval: u64,
    pub agents: Vec<(String, String)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            auto_refresh: true,
            refresh_interval: 10,
            agents: vec![
                ("claude".to_string(), "claude".to_string()),
                ("codex".to_string(), "codex".to_string()),
                ("kimi-cli".to_string(), "kimi".to_string()),
                ("gemini-cli".to_string(), "gemini".to_string()),
                ("opencode".to_string(), "opencode".to_string()),
                ("aider".to_string(), "aider".to_string()),
                ("cursor".to_string(), "cursor".to_string()),
            ],
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
        path.push("pad");
        path.push("config.toml");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let table: HashMap<String, toml::Value> = match toml::from_str(&content) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };

        let mut config = Self::default();

        if let Some(toml::Value::String(theme)) = table.get("theme") {
            config.theme = theme.clone();
        }
        if let Some(toml::Value::Boolean(auto)) = table.get("auto_refresh") {
            config.auto_refresh = *auto;
        }
        if let Some(toml::Value::Integer(interval)) = table.get("refresh_interval") {
            config.refresh_interval = *interval as u64;
        }
        if let Some(toml::Value::Array(agents)) = table.get("agents") {
            let mut parsed = Vec::new();
            for agent in agents {
                if let toml::Value::Table(t) = agent {
                    if let (Some(toml::Value::String(name)), Some(toml::Value::String(cmd))) =
                        (t.get("name"), t.get("cmd"))
                    {
                        parsed.push((name.clone(), cmd.clone()));
                    }
                }
            }
            if !parsed.is_empty() {
                config.agents = parsed;
            }
        }

        config
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut content = String::new();
        content.push_str(&format!("theme = \"{}\"\n", self.theme));
        content.push_str(&format!("auto_refresh = {}\n", self.auto_refresh));
        content.push_str(&format!("refresh_interval = {}\n", self.refresh_interval));
        content.push_str("\n");
        for (name, cmd) in &self.agents {
            content.push_str(&format!(
                "[[agents]]\nname = \"{}\"\ncmd = \"{}\"\n\n",
                name, cmd
            ));
        }

        let _ = std::fs::write(&path, content);
    }
}
