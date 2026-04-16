use super::*;

impl Theme {
    pub(super) fn default_theme() -> Self {
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

    pub(super) fn dark() -> Self {
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

    pub(super) fn dracula() -> Self {
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

    pub(super) fn nord() -> Self {
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

    pub(super) fn catppuccin() -> Self {
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

    pub(super) fn gruvbox() -> Self {
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

    pub(super) fn tokyo_night() -> Self {
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
}
