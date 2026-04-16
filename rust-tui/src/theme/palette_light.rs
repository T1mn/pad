use super::*;

impl Theme {
    pub(super) fn monokai() -> Self {
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

    pub(super) fn solarized_dark() -> Self {
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

    pub(super) fn rose_pine() -> Self {
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

    pub(super) fn solarized_light() -> Self {
        Self {
            name: "solarized-light",
            bg: Color::Rgb(253, 246, 227),
            fg: Color::Rgb(88, 110, 117),
            accent: Color::Rgb(38, 139, 210),
            highlight_bg: Color::Rgb(238, 232, 213),
            highlight_fg: Color::Rgb(7, 54, 66),
            border: Color::Rgb(147, 161, 161),
            border_focused: Color::Rgb(38, 139, 210),
            status_fg: Color::Rgb(131, 148, 150),
            error: Color::Rgb(220, 50, 47),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            comment: Color::Rgb(131, 148, 150),
            keyword: Color::Rgb(108, 113, 196),
            string_color: Color::Rgb(42, 161, 152),
            number: Color::Rgb(211, 54, 130),
            mode_normal_bg: Color::Rgb(38, 139, 210),
            mode_search_bg: Color::Rgb(181, 137, 0),
            mode_tree_bg: Color::Rgb(133, 153, 0),
        }
    }

    pub(super) fn one_dark() -> Self {
        Self {
            name: "one-dark",
            bg: Color::Rgb(40, 44, 52),
            fg: Color::Rgb(171, 178, 191),
            accent: Color::Rgb(97, 175, 239),
            highlight_bg: Color::Rgb(62, 68, 81),
            highlight_fg: Color::Rgb(229, 192, 123),
            border: Color::Rgb(92, 99, 112),
            border_focused: Color::Rgb(97, 175, 239),
            status_fg: Color::Rgb(92, 99, 112),
            error: Color::Rgb(224, 108, 117),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            comment: Color::Rgb(92, 99, 112),
            keyword: Color::Rgb(198, 120, 221),
            string_color: Color::Rgb(152, 195, 121),
            number: Color::Rgb(209, 154, 102),
            mode_normal_bg: Color::Rgb(97, 175, 239),
            mode_search_bg: Color::Rgb(229, 192, 123),
            mode_tree_bg: Color::Rgb(152, 195, 121),
        }
    }

    pub(super) fn github_light() -> Self {
        Self {
            name: "github-light",
            bg: Color::Rgb(255, 255, 255),
            fg: Color::Rgb(36, 41, 46),
            accent: Color::Rgb(3, 102, 214),
            highlight_bg: Color::Rgb(241, 248, 255),
            highlight_fg: Color::Rgb(36, 41, 46),
            border: Color::Rgb(225, 228, 232),
            border_focused: Color::Rgb(3, 102, 214),
            status_fg: Color::Rgb(88, 96, 105),
            error: Color::Rgb(215, 58, 73),
            success: Color::Rgb(40, 167, 69),
            warning: Color::Rgb(249, 130, 108),
            comment: Color::Rgb(106, 115, 125),
            keyword: Color::Rgb(215, 58, 73),
            string_color: Color::Rgb(3, 47, 98),
            number: Color::Rgb(0, 92, 197),
            mode_normal_bg: Color::Rgb(3, 102, 214),
            mode_search_bg: Color::Rgb(249, 130, 108),
            mode_tree_bg: Color::Rgb(40, 167, 69),
        }
    }

    pub(super) fn github_dark() -> Self {
        Self {
            name: "github-dark",
            bg: Color::Rgb(13, 17, 23),
            fg: Color::Rgb(201, 209, 217),
            accent: Color::Rgb(88, 166, 255),
            highlight_bg: Color::Rgb(22, 27, 34),
            highlight_fg: Color::Rgb(240, 246, 252),
            border: Color::Rgb(48, 54, 61),
            border_focused: Color::Rgb(88, 166, 255),
            status_fg: Color::Rgb(139, 148, 158),
            error: Color::Rgb(248, 81, 73),
            success: Color::Rgb(63, 185, 80),
            warning: Color::Rgb(210, 153, 34),
            comment: Color::Rgb(139, 148, 158),
            keyword: Color::Rgb(255, 123, 114),
            string_color: Color::Rgb(165, 214, 255),
            number: Color::Rgb(121, 192, 255),
            mode_normal_bg: Color::Rgb(88, 166, 255),
            mode_search_bg: Color::Rgb(210, 153, 34),
            mode_tree_bg: Color::Rgb(63, 185, 80),
        }
    }

    pub(super) fn everforest() -> Self {
        Self {
            name: "everforest",
            bg: Color::Rgb(45, 53, 59),
            fg: Color::Rgb(211, 198, 170),
            accent: Color::Rgb(167, 192, 128),
            highlight_bg: Color::Rgb(61, 72, 77),
            highlight_fg: Color::Rgb(211, 198, 170),
            border: Color::Rgb(71, 82, 88),
            border_focused: Color::Rgb(167, 192, 128),
            status_fg: Color::Rgb(133, 146, 137),
            error: Color::Rgb(230, 126, 128),
            success: Color::Rgb(167, 192, 128),
            warning: Color::Rgb(219, 188, 127),
            comment: Color::Rgb(133, 146, 137),
            keyword: Color::Rgb(214, 153, 182),
            string_color: Color::Rgb(167, 192, 128),
            number: Color::Rgb(214, 153, 182),
            mode_normal_bg: Color::Rgb(167, 192, 128),
            mode_search_bg: Color::Rgb(219, 188, 127),
            mode_tree_bg: Color::Rgb(131, 192, 146),
        }
    }
}
